import type { FrameDto, ReadingDto } from "../../shared/contracts/monitor";
import { emptyReading } from "./useMonitorViewModel";

/** Adapt independent metrics to the existing chart's five value slots; no sampling here. */
export function plotFrame(
  at: number,
  elapsed: number,
  generation: string,
  values: {
    cpu?: ReadingDto;
    memory?: ReadingDto;
    download?: ReadingDto;
    upload?: ReadingDto;
  },
): FrameDto {
  return {
    at_ms: at,
    elapsed_ms: elapsed,
    cursor: String(elapsed),
    generation,
    network_generation: generation,
    network_id: null,
    cpu: values.cpu ?? emptyReading,
    memory: values.memory ?? emptyReading,
    cpu_temperature: emptyReading,
    download: values.download ?? emptyReading,
    upload: values.upload ?? emptyReading,
    memory_used: "—",
    memory_total: "—",
    gpus: [],
  };
}
