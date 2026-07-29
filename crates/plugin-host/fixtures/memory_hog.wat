;; Tries to grow its linear memory far past any reasonable limit.
;; Proves the sandbox's memory limit (a `ResourceLimiter`, not fuel or
;; the epoch clock) actually rejects the growth.
(module
  (memory (export "memory") 1)
  (func (export "grow_a_lot") (result i32)
    i32.const 10000
    memory.grow))
