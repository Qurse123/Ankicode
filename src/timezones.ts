const BASE_TIMEZONES = [
  "UTC",
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "America/Los_Angeles",
  "America/Toronto",
  "America/Vancouver",
  "America/Sao_Paulo",
  "Europe/London",
  "Europe/Paris",
  "Europe/Berlin",
  "Europe/Madrid",
  "Asia/Tokyo",
  "Asia/Shanghai",
  "Asia/Singapore",
  "Asia/Kolkata",
  "Asia/Dubai",
  "Australia/Sydney",
  "Pacific/Auckland",
] as const;

export function detectSystemTimezone(): string {
  try {
    const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (zone && typeof zone === "string") {
      return zone;
    }
  } catch {
    // fall through
  }
  return "America/New_York";
}

export function timezoneOptions(preferred?: string): string[] {
  const values = new Set<string>(BASE_TIMEZONES);
  values.add(detectSystemTimezone());
  if (preferred) {
    values.add(preferred);
  }
  return Array.from(values).sort((left, right) => left.localeCompare(right));
}
