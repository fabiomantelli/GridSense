// Mirrors voltcase-core's comtrade::model types (serde field names are snake_case,
// unchanged by serde-wasm-bindgen). Keep in sync by hand until the surface grows
// enough to justify generating this from the Rust types.

import type { ComtradeHandle } from '../wasm-pkg/voltcase_wasm';

export type Revision = 'Y1991' | 'Y1999' | 'Y2013';
export type DatFormat = 'Ascii' | 'Binary16' | 'Binary32' | 'Float32';

export interface AnalogChannelDef {
  index: number;
  id: string;
  phase: string | null;
  circuit_component: string | null;
  units: string;
  a: number;
  b: number;
  skew: number;
  min: number;
  max: number;
  primary: number;
  secondary: number;
  ps: string;
}

export interface DigitalChannelDef {
  index: number;
  id: string;
  phase: string | null;
  circuit_component: string | null;
  normal_state: boolean;
}

export interface SampleRateSegment {
  samp_hz: number;
  end_sample: number;
}

export interface CfgFile {
  station_name: string;
  device_id: string;
  revision: Revision;
  analog_channels: AnalogChannelDef[];
  digital_channels: DigitalChannelDef[];
  line_frequency: number;
  sample_rates: SampleRateSegment[];
  total_samples: number;
  timestamp_start_raw: string;
  timestamp_trigger_raw: string;
  /** Microseconds since the Unix epoch, parsed from timestamp_start_raw; null if
   *  that field was missing/malformed. Absolute per-sample time is
   *  start_epoch_us + (timestamps_us from the WASM handle). */
  start_epoch_us: number | null;
  dat_format: DatFormat;
  time_multiplier: number;
}

// Mirrors voltcase-core's analysis::facts / event_correlation types. Serde's default
// enum representation: unit-only enums (Phase) serialize as bare strings; mixed
// enums (FaultKind) are externally tagged, tuple variants as arrays.
export type Phase = 'A' | 'B' | 'C';

export type FaultKind =
  | 'ThreePhase'
  | 'Unclassified'
  | { PhaseToGround: Phase }
  | { PhaseToPhase: [Phase, Phase] };

export interface EventClassification {
  kind: FaultKind;
  onset_sample: number;
  onset_time_us: number;
  involved_group_label: string;
  current_multiple: number | null;
  breaker_channel_id: string | null;
  time_to_trip_us: number | null;
}

export interface RecordSummary {
  station_name: string;
  device_id: string;
  sample_count: number;
  duration_us: number;
  line_frequency: number;
}

export interface ChannelSummary {
  index: number;
  id: string;
  units: string;
  min: number;
  max: number;
  mean: number;
  rms: number;
}

export interface SequenceGroupResult {
  group_label: string;
  units: string;
  sample_index: number;
  zero_magnitude: number;
  positive_magnitude: number;
  negative_magnitude: number;
}

export interface AnalysisFacts {
  record_summary: RecordSummary;
  channel_summaries: ChannelSummary[];
  sequence_component_groups: SequenceGroupResult[];
  events: EventClassification[];
}

/** One loaded COMTRADE record and its precomputed analysis, keyed by file stem. */
export interface Session {
  stem: string;
  metadata: CfgFile;
  handle: ComtradeHandle;
  facts: AnalysisFacts;
}
