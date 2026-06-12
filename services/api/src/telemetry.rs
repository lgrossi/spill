use opentelemetry::trace::TraceContextExt as _;
use std::io::{self, Write};

/// Buffers one JSON log line written by `tracing_subscriber::fmt`.
/// On drop, injects `dd.trace_id` and `dd.span_id` (captured at creation
/// time while the active OTel span is still on the thread-local context)
/// and flushes to the wrapped inner writer.
pub struct DdInjectWriter<W: Write> {
    inner: W,
    buf: Vec<u8>,
    dd_trace_id: Option<String>,
    dd_span_id: Option<String>,
}

impl<W: Write> DdInjectWriter<W> {
    fn new(inner: W) -> Self {
        // Capture the OTel span context now.  tracing-opentelemetry attaches
        // the OTel context to the thread-local when a tracing span is entered,
        // so Context::current() is valid during on_event / make_writer_for.
        let cx = opentelemetry::Context::current();
        let span_ref = cx.span();
        let span_ctx = span_ref.span_context();

        let (dd_trace_id, dd_span_id) = if span_ctx.is_valid() {
            let trace_bytes = span_ctx.trace_id().to_bytes();
            // DD uses the lower 64 bits of the 128-bit OTel trace ID.
            let dd_trace_id =
                u64::from_be_bytes(trace_bytes[8..16].try_into().unwrap()).to_string();
            let dd_span_id = u64::from_be_bytes(span_ctx.span_id().to_bytes()).to_string();
            (Some(dd_trace_id), Some(dd_span_id))
        } else {
            (None, None)
        };

        Self {
            inner,
            buf: Vec::with_capacity(256),
            dd_trace_id,
            dd_span_id,
        }
    }

    fn flush_buf(&mut self) -> io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let raw = std::mem::take(&mut self.buf);

        let out: Vec<u8> = match (&self.dd_trace_id, &self.dd_span_id) {
            (Some(tid), Some(sid)) => {
                let s = std::str::from_utf8(&raw).unwrap_or("");
                match serde_json::from_str::<serde_json::Value>(s.trim_end()) {
                    Ok(mut v) => {
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert(
                                "dd.trace_id".to_owned(),
                                serde_json::Value::String(tid.clone()),
                            );
                            obj.insert(
                                "dd.span_id".to_owned(),
                                serde_json::Value::String(sid.clone()),
                            );
                        }
                        match serde_json::to_vec(&v) {
                            Ok(mut bytes) => {
                                bytes.push(b'\n');
                                bytes
                            }
                            // Serialisation failure is extremely unlikely; fall back.
                            Err(_) => raw,
                        }
                    }
                    // Not valid JSON (e.g. plain-text fallback); pass through.
                    Err(_) => raw,
                }
            }
            // No active OTel span — pass through unchanged.
            _ => raw,
        };

        self.inner.write_all(&out)?;
        self.inner.flush()
    }
}

impl<W: Write> Write for DdInjectWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Buffer everything; the actual write+inject happens on drop/flush.
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buf()
    }
}

impl<W: Write> Drop for DdInjectWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush_buf();
    }
}

/// A `MakeWriter` that wraps another `MakeWriter` (e.g. `std::io::stdout`)
/// and produces `DdInjectWriter`s that inject Datadog trace-correlation fields
/// into every JSON log line.
pub struct DdMakeWriter<M>(pub M);

impl<'a, M> tracing_subscriber::fmt::MakeWriter<'a> for DdMakeWriter<M>
where
    M: tracing_subscriber::fmt::MakeWriter<'a>,
{
    type Writer = DdInjectWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        DdInjectWriter::new(self.0.make_writer())
    }

    fn make_writer_for(&'a self, meta: &tracing::Metadata<'_>) -> Self::Writer {
        DdInjectWriter::new(self.0.make_writer_for(meta))
    }
}
