import React from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  CheckCircle2,
  AlertCircle,
  Info,
  AlertTriangle,
  X,
} from "lucide-react";
import { useApp } from "../stores/AppContext";

export const ToastContainer: React.FC = () => {
  const { toasts, dismissToast } = useApp();

  return (
    <div
      aria-live="polite"
      className="fixed bottom-4 right-4 z-50 flex flex-col gap-2.5 max-w-sm w-full pointer-events-none"
    >
      <AnimatePresence>
        {toasts.map((toast) => (
          <motion.div
            key={toast.id}
            initial={{ opacity: 0, y: 16, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 8, scale: 0.96 }}
            transition={{ duration: 0.22, ease: [0.2, 0, 0, 1] }}
            className="pointer-events-auto flex items-start gap-3 p-3.5 rounded-xl bg-surface-elevated border border-border-strong shadow-elevated"
          >
            <div className="mt-0.5 shrink-0">
              {toast.type === "success" && (
                <CheckCircle2 className="w-4 h-4 text-emerald-500" />
              )}
              {toast.type === "error" && (
                <AlertCircle className="w-4 h-4 text-rose-500" />
              )}
              {toast.type === "warning" && (
                <AlertTriangle className="w-4 h-4 text-amber-500" />
              )}
              {toast.type === "info" && (
                <Info className="w-4 h-4 text-indigo-500" />
              )}
            </div>

            <div className="flex-1 min-w-0">
              <div className="text-xs font-semibold text-zinc-900 dark:text-zinc-100">
                {toast.title}
              </div>
              {toast.description && (
                <div className="text-xs text-zinc-500 dark:text-zinc-400 truncate mt-0.5">
                  {toast.description}
                </div>
              )}
              {toast.actionLabel && toast.onAction && (
                <button
                  type="button"
                  onClick={() => {
                    toast.onAction?.();
                    dismissToast(toast.id);
                  }}
                  className="mt-2 inline-flex items-center text-xs font-semibold text-indigo-600 dark:text-indigo-400 hover:underline"
                >
                  {toast.actionLabel} →
                </button>
              )}
            </div>

            <button
              type="button"
              onClick={() => dismissToast(toast.id)}
              aria-label="Dismiss notification"
              className="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200 transition-colors"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
};
