;; Declares a host import the sandbox never links (no plugin gets any
;; host bindings — network, filesystem, etc. — by default). Proves
;; default-deny: instantiating this against an empty imports list must
;; fail rather than silently granting the call.
(module
  (import "env" "forbidden_network_call" (func $forbidden (result i32)))
  (func (export "identify") (result i32)
    call $forbidden))
