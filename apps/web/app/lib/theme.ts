import { cookies } from "next/headers";

export const THEME_COOKIE = "spillio-theme";
export type Theme = "light" | "dark";

export async function readTheme(): Promise<Theme> {
  const jar = await cookies();
  const raw = jar.get(THEME_COOKIE)?.value;
  return raw === "dark" ? "dark" : "light";
}
