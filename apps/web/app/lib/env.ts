// Validated env vars with lazy proxy. Fails at runtime (not build time).
// Build-time collection of page data runs without most runtime vars.
import { z } from 'zod';

const schema = z.object({
  NODE_ENV: z.enum(['development', 'test', 'production']).default('development'),
  SPILLIO_AUTH_MODE: z.enum(['proxy', 'local']).optional(),
  SPILLIO_DIRECTORY_URL: z.string().url().optional(),
  SPILLIO_DIRECTORY_IAP_SA: z.string().optional(),
  SPILLIO_DIRECTORY_IAP_AUDIENCE: z.string().optional(),
});

export type Env = z.infer<typeof schema>;

function parseEnv(): Env {
  const result = schema.safeParse(process.env);
  if (!result.success) {
    throw new Error(`Invalid environment variables:\n${result.error.toString()}`);
  }
  return result.data;
}

export const env: Env = new Proxy({} as Env, {
  get(_target, key: string) {
    return parseEnv()[key as keyof Env];
  },
});
