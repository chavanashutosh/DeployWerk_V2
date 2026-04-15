## Deploy strategies (current behavior)

DeployWerk V2 exposes a `deploy_strategy` setting on applications and deploy jobs. The **current deploy worker behavior is the same for all strategies**: it performs a “standard” container replacement on the destination (SSH or local Docker), and it does **not** implement traffic shifting semantics yet.

### Status: implemented vs staged

- **Implemented today**
  - `standard`: Pull/build (when configured), stop/replace container, write logs, mark job success/failure.
  - `blue_green`, `canary`, `rolling`: Accepted and stored as metadata, logged in job output, and echoed in APIs/UI.

- **Staged / not implemented yet**
  - **Blue/green** traffic cutover (two versions running, atomic switch, keep old warm).
  - **Canary** weighted routing and promotion/abort logic based on health/metrics.
  - **Rolling** orchestration across groups with max-unavailable.

### Where this is implemented

- Strategy is validated and stored on create/update of an application in:
  - `crates/deploywerk-api/src/applications.rs`
- Strategy is copied to deploy jobs at enqueue time and logged by the worker:
  - `crates/deploywerk-api/src/applications.rs` (`execute_deploy_job` logs a note that advanced routing is staged separately)

### Why keep the field now

The field establishes a stable API/UI contract and schema. The actual semantics will be expanded as edge/routing and observability capabilities mature (see `docs/ENTERPRISE_GAPS.md` sections on deploy lifecycle and networking/edge).

