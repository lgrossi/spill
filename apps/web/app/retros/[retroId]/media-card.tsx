"use client";

import { useRef, useState } from "react";

export function BoardMedia({ alt, src }: { alt: string; src: string }) {
  if (isVideo(src)) {
    return <VideoMedia alt={alt} src={src} />;
  }

  return <img className="media-image gif-image" src={src} alt={alt} loading="lazy" />;
}

function VideoMedia({ alt, src }: { alt: string; src: string }) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [muted, setMuted] = useState(true);

  return (
    <div className="media-video-frame">
      <video
        ref={videoRef}
        aria-label={alt}
        autoPlay
        className="media-video gif-image"
        loop
        muted={muted}
        playsInline
        src={src}
      />
      <button
        aria-label={muted ? "Turn media sound on" : "Turn media sound off"}
        className="media-sound-toggle"
        type="button"
        onClick={() => {
          const nextMuted = !muted;
          setMuted(nextMuted);
          if (videoRef.current) {
            videoRef.current.muted = nextMuted;
            if (!nextMuted) {
              void videoRef.current.play();
            }
          }
        }}
      >
        {muted ? "off" : "on"}
      </button>
    </div>
  );
}

function isVideo(src: string) {
  return /\.(mp4|webm)(\?|$)/i.test(src);
}
