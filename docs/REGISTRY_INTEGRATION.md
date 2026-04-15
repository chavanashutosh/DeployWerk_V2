## Container registry integration (current approach)

DeployWerk V2 does **not** bundle an OCI registry today. Applications reference images by tag/digest (e.g. `ghcr.io/org/app:sha-...`) and the deploy worker pulls those images on the target host.

### Recommended pattern

1. **Build in CI** and push to your registry (GHCR, ECR, GCR, Docker Hub, self-hosted, etc.). Prefer immutable tags (commit SHA) or digests.
2. **Scan in CI** (Trivy/Grype/Snyk/etc.) and fail the pipeline on policy violations (e.g. Critical CVEs).
3. Configure the DeployWerk application’s `docker_image` to the pushed image reference.
4. Trigger deploys via:
   - The UI (manual deploy)
   - GitHub/GitLab push hooks (auto deploy)
   - The API/CLI

### What is pending

- First-class registry integration with on-push scanning and deploy gates is tracked as a P1 gap in `docs/ENTERPRISE_GAPS.md` under “Container registry”.

