;; An unconditional infinite loop — proves the sandbox's fuel limit
;; actually halts a runaway plugin rather than just documenting a CPU
;; time limit.
(module
  (func (export "spin")
    (loop $continue
      br $continue)))
