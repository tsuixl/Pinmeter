import type { AlertsConfigDto } from "../contracts/monitor";

export function defaultAlertsConfig(): AlertsConfigDto {
  return {
    cpu: {
      enabled: false,
      threshold_percent: 90,
      duration_seconds: 30,
      cooldown_seconds: 300,
    },
    memory: {
      enabled: false,
      threshold_percent: 90,
      duration_seconds: 30,
      cooldown_seconds: 300,
    },
    quiet: { enabled: false, start_minute: 1320, end_minute: 480 },
  };
}
