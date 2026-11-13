import { create } from "zustand";
import type {
  JobRecord,
  ProgressPayload,
  QueueJobStatus,
  RateLimitTrippedPayload,
} from "../types";

export type QueueFilterTab = "all" | "active" | "queued" | "paused" | "failed";

interface QueueState {
  jobs: JobRecord[];
  telemetryMap: Record<string, ProgressPayload>;
  activeFilter: QueueFilterTab;
  rateLimitAlert: RateLimitTrippedPayload | null;

  setJobs: (jobs: JobRecord[]) => void;
  upsertJob: (job: JobRecord) => void;
  applyProgressTelemetry: (payload: ProgressPayload) => void;
  setActiveFilter: (tab: QueueFilterTab) => void;
  setRateLimitAlert: (alert: RateLimitTrippedPayload | null) => void;
  getFilteredJobs: () => JobRecord[];
  getAggregateSpeedBps: () => number;
}

export const useQueueStore = create<QueueState>((set, get) => ({
  jobs: [],
  telemetryMap: {},
  activeFilter: "all",
  rateLimitAlert: null,

  setJobs: (jobs) => set({ jobs }),

  upsertJob: (job) =>
    set((state) => {
      const exists = state.jobs.some((j) => j.id === job.id);
      if (exists) {
        return {
          jobs: state.jobs.map((j) => (j.id === job.id ? job : j)),
        };
      }
      return {
        jobs: [job, ...state.jobs],
      };
    }),

  applyProgressTelemetry: (payload) =>
    set((state) => {
      const mappedStatus: QueueJobStatus | null =
        payload.status === "completed"
          ? "completed"
          : payload.status === "failed"
          ? "failed"
          : payload.status === "paused" || payload.status === "rate_limited"
          ? "paused"
          : "running";

      return {
        telemetryMap: {
          ...state.telemetryMap,
          [payload.job_id]: payload,
        },
        jobs: state.jobs.map((j) =>
          j.id === payload.job_id
            ? {
                ...j,
                status: mappedStatus ?? j.status,
                progress_percent: payload.percentage,
                downloaded_bytes: payload.downloaded_bytes,
                total_bytes: payload.total_bytes ?? j.total_bytes,
              }
            : j
        ),
      };
    }),

  setActiveFilter: (activeFilter) => set({ activeFilter }),

  setRateLimitAlert: (rateLimitAlert) => set({ rateLimitAlert }),

  getFilteredJobs: () => {
    const { jobs, activeFilter } = get();
    switch (activeFilter) {
      case "active":
        return jobs.filter((j) => j.status === "running");
      case "queued":
        return jobs.filter((j) => j.status === "queued");
      case "paused":
        return jobs.filter((j) => j.status === "paused");
      case "failed":
        return jobs.filter((j) => j.status === "failed" || j.status === "cancelled");
      default:
        return jobs;
    }
  },

  getAggregateSpeedBps: () => {
    const { jobs, telemetryMap } = get();
    let sum = 0;
    for (const j of jobs) {
      if (j.status === "running") {
        const t = telemetryMap[j.id];
        if (t?.speed_bps) {
          sum += t.speed_bps;
        }
      }
    }
    return sum;
  },
}));
