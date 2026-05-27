/** Reads a FormData field as a string, returning "" when absent. */
export function field(formData: FormData, name: string): string {
  return String(formData.get(name) ?? "");
}
