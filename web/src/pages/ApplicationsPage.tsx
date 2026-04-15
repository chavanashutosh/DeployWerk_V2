import { useEffect, useRef, useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { Box } from "lucide-react";
import {
  apiFetch,
  getToken,
  resolveApiUrl,
  type Application,
  type ApplicationDetail,
  type Bootstrap,
  type DeployJob,
  type Destination,
  type Environment,
  type Project,
  type Team,
  type User,
} from "@/api";
import { useAuth } from "@/auth";
import { EmptyState, InlineError, LoadingBlock, PageHeader } from "@/components/ui";

function appMembershipRole(user: User | null, appId: string): "admin" | "viewer" | null {
  const m = user?.application_memberships?.find((x) => x.application_id === appId);
  return m?.role ?? null;
}

function appsPath(teamId: string, projectId: string, envId: string) {
  return `/api/v1/teams/${teamId}/projects/${projectId}/environments/${envId}/applications`;
}

export function ApplicationsPage() {
  const { user } = useAuth();
  const { teamId = "", projectId = "", environmentId = "" } = useParams();
  const [apps, setApps] = useState<Application[] | null>(null);
  const [destinations, setDestinations] = useState<Destination[]>([]);
  const [teams, setTeams] = useState<Team[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [dockerImage, setDockerImage] = useState("");
  const [destinationId, setDestinationId] = useState("");
  const [domainsStr, setDomainsStr] = useState("");
  const [gitUrl, setGitUrl] = useState("");
  const [gitFullName, setGitFullName] = useState("");
  const [autoDeployPush, setAutoDeployPush] = useState(false);
  const [prPreviewPush, setPrPreviewPush] = useState(false);
  const [branchPattern, setBranchPattern] = useState("main");
  const [buildFromGit, setBuildFromGit] = useState(false);
  const [gitBuildRef, setGitBuildRef] = useState("main");
  const [dockerfilePath, setDockerfilePath] = useState("Dockerfile");
  const [gitUrlEdit, setGitUrlEdit] = useState("");
  const [gitFullNameEdit, setGitFullNameEdit] = useState("");
  const [autoDeployEdit, setAutoDeployEdit] = useState(false);
  const [prPreviewEdit, setPrPreviewEdit] = useState(false);
  const [branchPatternEdit, setBranchPatternEdit] = useState("main");
  const [buildFromGitEdit, setBuildFromGitEdit] = useState(false);
  const [gitBuildRefEdit, setGitBuildRefEdit] = useState("main");
  const [dockerfilePathEdit, setDockerfilePathEdit] = useState("Dockerfile");
  const [pending, setPending] = useState(false);
  const [deployingId, setDeployingId] = useState<string | null>(null);
  const [rollingBackId, setRollingBackId] = useState<string | null>(null);
  const [logTail, setLogTail] = useState<{ appId: string; text: string } | null>(null);
  const logAbortRef = useRef<AbortController | null>(null);
  const [jobPoll, setJobPoll] = useState<{
    jobId: string;
    log: string;
    status: string;
    gitRef?: string | null;
    gitSha?: string | null;
    log_object_key?: string | null;
    artifact_manifest_key?: string | null;
  } | null>(null);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [detail, setDetail] = useState<ApplicationDetail | null>(null);
  const [envJson, setEnvJson] = useState("[]");
  const [runtimeVolumesJson, setRuntimeVolumesJson] = useState("[]");
  const [runtimeVolumesCreateJson, setRuntimeVolumesCreateJson] = useState("[]");
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [destEdit, setDestEdit] = useState("");
  const [domainsEdit, setDomainsEdit] = useState("");
  const [deployStrategyEdit, setDeployStrategyEdit] = useState("standard");
  const [requireDeployApprovalEdit, setRequireDeployApprovalEdit] = useState(false);
  const [preDeployHookEdit, setPreDeployHookEdit] = useState("");
  const [postDeployHookEdit, setPostDeployHookEdit] = useState("");
  const [pathCtx, setPathCtx] = useState<{ projectName: string; envName: string } | null>(null);

  const team = teams.find((t) => t.id === teamId);
  const orgOnly = !!team?.access_via_organization_admin;

  const canCreateApplication =
    !!user?.is_platform_admin || team?.role === "admin" || team?.role === "owner";

  const canEditApp = (appId: string) => {
    if (!user || !team) return false;
    if (user.is_platform_admin) return true;
    if (orgOnly) return appMembershipRole(user, appId) === "admin";
    return team.role === "admin" || team.role === "owner";
  };

  const canDeployApp = (appId: string) => {
    if (!user || !team) return false;
    if (user.is_platform_admin) return true;
    if (orgOnly) return appMembershipRole(user, appId) === "admin";
    return team.role === "member" || team.role === "admin" || team.role === "owner";
  };

  const viewOnlyUi =
    !!team &&
    !canCreateApplication &&
    apps !== null &&
    (apps.length === 0 || !apps.some((a) => canEditApp(a.id) || canDeployApp(a.id)));

  async function loadApps() {
    if (!teamId || !projectId || !environmentId) return;
    const list = await apiFetch<Application[]>(`${appsPath(teamId, projectId, environmentId)}`);
    setApps(list);
  }

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [t, b] = await Promise.all([
          apiFetch<Team[]>("/api/v1/teams"),
          apiFetch<Bootstrap>("/api/v1/bootstrap"),
        ]);
        if (!cancelled) {
          setTeams(t);
          setBootstrap(b);
        }
      } catch {
        if (!cancelled) {
          setTeams([]);
          setBootstrap(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!teamId || !projectId || !environmentId) return;
    let cancelled = false;
    (async () => {
      try {
        const [proj, env] = await Promise.all([
          apiFetch<Project>(`/api/v1/teams/${teamId}/projects/${projectId}`),
          apiFetch<Environment>(
            `/api/v1/teams/${teamId}/projects/${projectId}/environments/${environmentId}`,
          ),
        ]);
        if (!cancelled) setPathCtx({ projectName: proj.name, envName: env.name });
      } catch {
        if (!cancelled) setPathCtx(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId, projectId, environmentId]);

  useEffect(() => {
    return () => {
      logAbortRef.current?.abort();
    };
  }, []);

  function stopLogTail() {
    logAbortRef.current?.abort();
    logAbortRef.current = null;
    setLogTail(null);
  }

  async function startLogTail(appId: string) {
    if (!teamId || !projectId || !environmentId) return;
    stopLogTail();
    const ac = new AbortController();
    logAbortRef.current = ac;
    setLogTail({ appId, text: "(connecting…)\n" });
    const url = resolveApiUrl(
      `${appsPath(teamId, projectId, environmentId)}/${appId}/container-log-stream`,
    );
    try {
      const res = await fetch(url, {
        headers: { Authorization: `Bearer ${getToken() ?? ""}` },
        signal: ac.signal,
      });
      if (!res.ok) {
        const t = await res.text();
        setLogTail({ appId, text: `HTTP ${res.status}: ${t || res.statusText}` });
        return;
      }
      const reader = res.body?.getReader();
      if (!reader) {
        setLogTail({ appId, text: "No response body" });
        return;
      }
      const dec = new TextDecoder();
      let carry = "";
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        carry += dec.decode(value, { stream: true });
        const blocks = carry.split("\n\n");
        carry = blocks.pop() ?? "";
        for (const b of blocks) {
          const lines = b.split("\n");
          for (const line of lines) {
            const trimmed = line.replace(/^data:\s*/, "").trim();
            if (!trimmed) continue;
            try {
              const j = JSON.parse(trimmed) as { log?: string; error?: string };
              const piece = j.log ?? (j.error ? `Error: ${j.error}` : "");
              if (piece)
                setLogTail((prev) =>
                  prev && prev.appId === appId ? { appId, text: piece } : prev,
                );
            } catch {
              /* ignore partial JSON */
            }
          }
        }
      }
    } catch (e) {
      if (e instanceof Error && e.name === "AbortError") return;
      setLogTail({ appId, text: e instanceof Error ? e.message : "Stream failed" });
    }
  }

  useEffect(() => {
    if (!teamId || !projectId || !environmentId) return;
    let cancelled = false;
    (async () => {
      try {
        const [list, dest] = await Promise.all([
          apiFetch<Application[]>(`${appsPath(teamId, projectId, environmentId)}`),
          apiFetch<Destination[]>(`/api/v1/teams/${teamId}/destinations`),
        ]);
        if (!cancelled) {
          setApps(list);
          setDestinations(dest);
          setErr(null);
        }
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Failed to load");
          setApps(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId, projectId, environmentId]);

  useEffect(() => {
    if (!detailId || !teamId || !projectId || !environmentId) {
      setDetail(null);
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const d = await apiFetch<ApplicationDetail>(
          `${appsPath(teamId, projectId, environmentId)}/${detailId}`,
        );
        if (!cancelled) {
          setDetail(d);
          setEnvJson(
            JSON.stringify(
              d.env_vars.map((v) => ({
                key: v.key,
                value: v.value ?? "",
                is_secret: v.is_secret,
              })),
              null,
              2,
            ),
          );
          setRuntimeVolumesJson(JSON.stringify(d.runtime_volumes ?? [], null, 2));
        }
      } catch {
        if (!cancelled) setDetail(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [detailId, teamId, projectId, environmentId]);

  useEffect(() => {
    if (!detail) return;
    setGitUrlEdit(detail.git_repo_url ?? "");
    setGitFullNameEdit(detail.git_repo_full_name ?? "");
    setAutoDeployEdit(!!detail.auto_deploy_on_push);
    setPrPreviewEdit(!!detail.pr_preview_enabled);
    setBranchPatternEdit(detail.git_branch_pattern?.trim() || "main");
    setBuildFromGitEdit(!!detail.build_image_from_git);
    setGitBuildRefEdit(detail.git_build_ref?.trim() || "main");
    setDockerfilePathEdit(detail.dockerfile_path?.trim() || "Dockerfile");
    setDestEdit(detail.destination_id ?? "");
    setDomainsEdit((detail.domains ?? []).join("\n"));
    setDeployStrategyEdit(detail.deploy_strategy ?? "standard");
    setRequireDeployApprovalEdit(!!detail.require_deploy_approval);
    setPreDeployHookEdit(detail.pre_deploy_hook_url ?? "");
    setPostDeployHookEdit(detail.post_deploy_hook_url ?? "");
  }, [detail]);

  useEffect(() => {
    if (!jobPoll?.jobId || !teamId) return;
    let cancelled = false;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const run = async () => {
      try {
        const j = await apiFetch<DeployJob>(
          `/api/v1/teams/${teamId}/deploy-jobs/${jobPoll.jobId}`,
        );
        if (cancelled) return;
        setJobPoll({
          jobId: j.id,
          log: j.log,
          status: j.status,
          gitRef: j.git_ref,
          gitSha: j.git_sha,
          log_object_key: j.log_object_key ?? null,
          artifact_manifest_key: j.artifact_manifest_key ?? null,
        });
        if (j.status !== "succeeded" && j.status !== "failed") {
          timeout = setTimeout(() => void run(), 1500);
        }
      } catch {
        if (!cancelled) setJobPoll(null);
      }
    };
    void run();
    return () => {
      cancelled = true;
      if (timeout) clearTimeout(timeout);
    };
  }, [jobPoll?.jobId, teamId]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !projectId || !environmentId || !name.trim() || !dockerImage.trim()) return;
    setPending(true);
    setErr(null);
    try {
      let runtime_volumes: { name: string; container_path: string }[];
      try {
        const v = JSON.parse(runtimeVolumesCreateJson) as unknown;
        if (!Array.isArray(v)) throw new Error("not array");
        runtime_volumes = v as { name: string; container_path: string }[];
      } catch {
        setErr('Runtime volumes must be a JSON array of { "name", "container_path" }');
        setPending(false);
        return;
      }
      const domains = domainsStr
        .split(/[\n,]/)
        .map((s) => s.trim())
        .filter(Boolean);
      await apiFetch(`${appsPath(teamId, projectId, environmentId)}`, {
        method: "POST",
        body: JSON.stringify({
          name: name.trim(),
          slug: slug.trim() || undefined,
          docker_image: dockerImage.trim(),
          destination_id: destinationId || null,
          domains: domains.length ? domains : undefined,
          git_repo_url: gitUrl.trim() || null,
          git_repo_full_name: gitFullName.trim() || undefined,
          auto_deploy_on_push: autoDeployPush,
          pr_preview_enabled: prPreviewPush,
          git_branch_pattern: branchPattern.trim() || undefined,
          build_image_from_git: buildFromGit,
          git_build_ref: gitBuildRef.trim() || undefined,
          dockerfile_path: dockerfilePath.trim() || undefined,
          runtime_volumes: runtime_volumes.length ? runtime_volumes : undefined,
        }),
      });
      setName("");
      setSlug("");
      setDockerImage("");
      setDestinationId("");
      setDomainsStr("");
      setGitUrl("");
      setGitFullName("");
      setAutoDeployPush(false);
      setPrPreviewPush(false);
      setBranchPattern("main");
      setBuildFromGit(false);
      setGitBuildRef("main");
      setDockerfilePath("Dockerfile");
      setRuntimeVolumesCreateJson("[]");
      await loadApps();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Create failed");
    } finally {
      setPending(false);
    }
  }

  async function onDeploy(appId: string) {
    if (!teamId || !projectId || !environmentId) return;
    setDeployingId(appId);
    setErr(null);
    try {
      const res = await apiFetch<{ job_id: string; status: string }>(
        `${appsPath(teamId, projectId, environmentId)}/${appId}/deploy`,
        { method: "POST" },
      );
      setJobPoll({
        jobId: res.job_id,
        log: "",
        status: res.status,
        gitRef: null,
        gitSha: null,
        log_object_key: null,
        artifact_manifest_key: null,
      });
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Deploy failed");
    } finally {
      setDeployingId(null);
    }
  }

  async function onRollback(appId: string) {
    if (!teamId || !projectId || !environmentId) return;
    setRollingBackId(appId);
    setErr(null);
    try {
      const res = await apiFetch<{ job_id: string; status: string }>(
        `${appsPath(teamId, projectId, environmentId)}/${appId}/rollback`,
        { method: "POST" },
      );
      setJobPoll({
        jobId: res.job_id,
        log: "",
        status: res.status,
        gitRef: null,
        gitSha: null,
        log_object_key: null,
        artifact_manifest_key: null,
      });
      await loadApps();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Rollback failed");
    } finally {
      setRollingBackId(null);
    }
  }

  async function onSaveEnv(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !projectId || !environmentId || !detailId) return;
    setPending(true);
    setErr(null);
    try {
      let parsed: { key: string; value: string; is_secret: boolean }[];
      try {
        parsed = JSON.parse(envJson) as { key: string; value: string; is_secret: boolean }[];
        if (!Array.isArray(parsed)) throw new Error("not array");
      } catch {
        setErr("Env vars must be a JSON array of { key, value, is_secret }");
        setPending(false);
        return;
      }
      await apiFetch(`${appsPath(teamId, projectId, environmentId)}/${detailId}`, {
        method: "PATCH",
        body: JSON.stringify({ env_vars: parsed }),
      });
      setDetailId(null);
      await loadApps();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Save failed");
    } finally {
      setPending(false);
    }
  }

  async function onSaveRuntimeVolumes(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !projectId || !environmentId || !detailId) return;
    setPending(true);
    setErr(null);
    try {
      let parsed: { name: string; container_path: string }[];
      try {
        parsed = JSON.parse(runtimeVolumesJson) as { name: string; container_path: string }[];
        if (!Array.isArray(parsed)) throw new Error("not array");
      } catch {
        setErr('Runtime volumes must be a JSON array of { "name", "container_path" }');
        setPending(false);
        return;
      }
      await apiFetch(`${appsPath(teamId, projectId, environmentId)}/${detailId}`, {
        method: "PATCH",
        body: JSON.stringify({ runtime_volumes: parsed }),
      });
      await loadApps();
      const d = await apiFetch<ApplicationDetail>(
        `${appsPath(teamId, projectId, environmentId)}/${detailId}`,
      );
      setDetail(d);
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Save failed");
    } finally {
      setPending(false);
    }
  }

  async function onSaveMeta(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !projectId || !environmentId || !detailId) return;
    setPending(true);
    setErr(null);
    try {
      const domains = domainsEdit
        .split(/[\n,]/)
        .map((s) => s.trim())
        .filter(Boolean);
      await apiFetch(`${appsPath(teamId, projectId, environmentId)}/${detailId}`, {
        method: "PATCH",
        body: JSON.stringify({
          destination_id: destEdit || null,
          domains,
        }),
      });
      await loadApps();
      const d = await apiFetch<ApplicationDetail>(
        `${appsPath(teamId, projectId, environmentId)}/${detailId}`,
      );
      setDetail(d);
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Save failed");
    } finally {
      setPending(false);
    }
  }

  async function onSaveDeployPolicy(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !projectId || !environmentId || !detailId) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`${appsPath(teamId, projectId, environmentId)}/${detailId}`, {
        method: "PATCH",
        body: JSON.stringify({
          deploy_strategy: deployStrategyEdit.trim() || "standard",
          require_deploy_approval: requireDeployApprovalEdit,
          pre_deploy_hook_url: preDeployHookEdit.trim(),
          post_deploy_hook_url: postDeployHookEdit.trim(),
        }),
      });
      await loadApps();
      const d = await apiFetch<ApplicationDetail>(
        `${appsPath(teamId, projectId, environmentId)}/${detailId}`,
      );
      setDetail(d);
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Save failed");
    } finally {
      setPending(false);
    }
  }

  async function onSaveGit(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !projectId || !environmentId || !detailId) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`${appsPath(teamId, projectId, environmentId)}/${detailId}`, {
        method: "PATCH",
        body: JSON.stringify({
          git_repo_url: gitUrlEdit.trim() || null,
          git_repo_full_name: gitFullNameEdit.trim() ? gitFullNameEdit.trim() : null,
          auto_deploy_on_push: autoDeployEdit,
          pr_preview_enabled: prPreviewEdit,
          git_branch_pattern: branchPatternEdit.trim() || "main",
          build_image_from_git: buildFromGitEdit,
          git_build_ref: gitBuildRefEdit.trim() || "main",
          dockerfile_path: dockerfilePathEdit.trim() || "Dockerfile",
        }),
      });
      await loadApps();
      const d = await apiFetch<ApplicationDetail>(
        `${appsPath(teamId, projectId, environmentId)}/${detailId}`,
      );
      setDetail(d);
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Save failed");
    } finally {
      setPending(false);
    }
  }

  if (!teamId || !projectId || !environmentId) {
    return <p className="dw-muted">Missing team, project, or environment.</p>;
  }

  return (
    <div className="space-y-8">
      <PageHeader
        icon={<Box className="h-6 w-6" strokeWidth={1.75} />}
        title="Applications"
        description={
          <>
            <Link
              to={`/app/teams/${teamId}/projects/${projectId}/environments`}
              className="font-medium text-brand-600 hover:text-brand-700"
            >
              Environments
            </Link>
            <span className="mx-1.5 text-slate-400">/</span>
            <span className="font-medium text-slate-800">{pathCtx?.envName ?? "…"}</span>
            <span className="mx-1.5 text-slate-400">·</span>
            <span className="text-slate-600">Project {pathCtx?.projectName ?? "…"}</span>
            {viewOnlyUi && (
              <span className="ml-2 rounded-md bg-slate-100 px-2 py-0.5 text-xs font-medium text-slate-600">
                View only
              </span>
            )}
          </>
        }
      />

      <InlineError message={err} />

      {apps === null && !err && <LoadingBlock label="Loading applications…" />}

      <div className="dw-card p-6 sm:p-8">
        {jobPoll && (
          <div className="mb-6 rounded-lg border border-slate-200 bg-slate-50 p-4">
            <p className="text-xs font-medium text-slate-500">
              Deploy job {jobPoll.jobId} — {jobPoll.status}
              {jobPoll.gitRef && (
                <span className="ml-2 font-mono text-slate-600">
                  {jobPoll.gitRef}
                  {jobPoll.gitSha ? ` @ ${jobPoll.gitSha.slice(0, 7)}` : ""}
                </span>
              )}
            </p>
            {(jobPoll.status === "succeeded" || jobPoll.status === "failed") &&
              (jobPoll.log_object_key || jobPoll.artifact_manifest_key) && (
                <p className="mt-2 text-xs text-slate-600">
                  Object storage:{" "}
                  {jobPoll.log_object_key && (
                    <span className="font-mono text-slate-800">log {jobPoll.log_object_key}</span>
                  )}
                  {jobPoll.log_object_key && jobPoll.artifact_manifest_key ? " · " : null}
                  {jobPoll.artifact_manifest_key && (
                    <span className="font-mono text-slate-800">
                      manifest {jobPoll.artifact_manifest_key}
                    </span>
                  )}
                </p>
              )}
            <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap font-mono text-xs text-slate-800">
              {jobPoll.log || "…"}
            </pre>
          </div>
        )}
        {apps && apps.length === 0 && (
          <EmptyState icon={Box} title="No applications yet">
            {canCreateApplication
              ? "Create an application below: pick an image or connect Git, assign a destination, then deploy."
              : "Nothing deployed here yet. Ask a team owner or admin to add an application."}
          </EmptyState>
        )}
        {apps && apps.length > 0 && (
          <ul className="mt-6 divide-y divide-slate-100">
            {apps.map((a) => (
              <li key={a.id} className="flex flex-wrap items-center justify-between gap-3 py-4 first:pt-0">
                <div>
                  <p className="font-medium text-slate-900">{a.name}</p>
                  <p className="font-mono text-sm text-slate-500">{a.docker_image}</p>
                  {a.destination_id && (
                    <p className="text-xs text-slate-500">
                      Destination:{" "}
                      {destinations.find((d) => d.id === a.destination_id)?.name ?? a.destination_id}
                    </p>
                  )}
                  {a.auto_deploy_on_push && a.git_repo_full_name && (
                    <p className="text-xs text-emerald-700">
                      Git auto-deploy: {a.git_repo_full_name} [{a.git_branch_pattern ?? "main"}]
                    </p>
                  )}
                  {a.pr_preview_enabled && a.git_repo_full_name && (
                    <p className="text-xs text-violet-700">PR preview deploys enabled (GitHub App webhook)</p>
                  )}
                  {a.build_image_from_git && (
                    <p className="text-xs text-sky-700">
                      Deploy builds image from Git ({a.git_build_ref ?? "main"}, {a.dockerfile_path ?? "Dockerfile"})
                    </p>
                  )}
                  {(a.last_deployed_image || a.previous_deployed_image) && (
                    <p className="text-xs font-mono text-slate-600">
                      {a.last_deployed_image && (
                        <span className="mr-2">Last run: {a.last_deployed_image}</span>
                      )}
                      {a.previous_deployed_image && (
                        <span>Prior: {a.previous_deployed_image}</span>
                      )}
                    </p>
                  )}
                  {a.auto_hostname && (
                    <p className="text-xs font-mono text-sky-800">
                      Provisioned URL: https://{a.auto_hostname}
                    </p>
                  )}
                </div>
                <div className="flex flex-wrap gap-2">
                  {canEditApp(a.id) && (
                    <button
                      type="button"
                      onClick={() => setDetailId(detailId === a.id ? null : a.id)}
                      className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm text-slate-700"
                    >
                      {detailId === a.id ? "Close" : "Env / meta"}
                    </button>
                  )}
                  <button
                    type="button"
                    disabled={!canDeployApp(a.id) || !!deployingId || !a.destination_id}
                    onClick={() => void onDeploy(a.id)}
                    className="rounded-lg bg-brand-600 px-3 py-1.5 text-sm font-semibold text-white hover:bg-brand-700 disabled:opacity-50"
                    title={
                      !canDeployApp(a.id)
                        ? "Deploy requires team membership or app admin on this application"
                        : !a.destination_id
                          ? "Set a destination on create or via Env / meta"
                          : ""
                    }
                  >
                    {deployingId === a.id ? "Deploying…" : "Deploy"}
                  </button>
                  <button
                    type="button"
                    disabled={
                      !canDeployApp(a.id) ||
                      !!rollingBackId ||
                      !!deployingId ||
                      !a.destination_id ||
                      !a.previous_deployed_image?.trim()
                    }
                    onClick={() => void onRollback(a.id)}
                    className="rounded-lg border border-amber-300 bg-amber-50 px-3 py-1.5 text-sm font-semibold text-amber-900 hover:bg-amber-100 disabled:opacity-50"
                    title={
                      !canDeployApp(a.id)
                        ? "Rollback requires team membership or app admin on this application"
                        : !a.previous_deployed_image?.trim()
                          ? "Need two successful deploys first"
                          : "Redeploy the previous image"
                    }
                  >
                    {rollingBackId === a.id ? "Rolling back…" : "Rollback"}
                  </button>
                  <button
                    type="button"
                    disabled={!a.destination_id}
                    onClick={() => void startLogTail(a.id)}
                    className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-50 disabled:opacity-50"
                  >
                    Container logs
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      {logTail && (
        <div className="rounded-xl border border-slate-200 bg-slate-950 p-4 shadow-sm">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-sm font-medium text-slate-200">Live container logs</h3>
            <button
              type="button"
              onClick={stopLogTail}
              className="rounded border border-slate-600 px-2 py-1 text-xs text-slate-300 hover:bg-slate-800"
            >
              Stop
            </button>
          </div>
          <pre className="max-h-[min(50vh,24rem)] overflow-auto whitespace-pre-wrap font-mono text-xs text-slate-100">
            {logTail.text}
          </pre>
        </div>
      )}

      {detailId && detail && canEditApp(detailId) && (
        <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
          <h2 className="text-lg font-semibold text-slate-900">Edit {detail.name}</h2>
          <p className="mt-1 text-sm text-slate-600">
            Secret values show only for admins/owners. Use JSON to replace all env vars.
          </p>
          {detail.auto_hostname && (
            <p className="mt-3 rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-sm text-sky-900">
              <strong>Provisioned hostname</strong> (stable for Traefik):{" "}
              <span className="font-mono">{detail.auto_hostname}</span>. Custom domains in the list below can point to
              the same edge; configure DNS at your provider.
            </p>
          )}
          <form className="mt-6 space-y-3 border-t border-slate-100 pt-6" onSubmit={onSaveMeta}>
            <h3 className="text-sm font-semibold text-slate-800">Destination &amp; domains</h3>
            <div>
              <label className="text-xs font-medium text-slate-500">Destination</label>
              <select
                value={destEdit}
                onChange={(e) => setDestEdit(e.target.value)}
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 text-sm"
              >
                <option value="">None</option>
                {destinations.map((d) => (
                  <option key={d.id} value={d.id}>
                    {d.name} ({d.slug})
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="text-xs font-medium text-slate-500">Domains (comma or newline)</label>
              <textarea
                value={domainsEdit}
                onChange={(e) => setDomainsEdit(e.target.value)}
                rows={3}
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              />
            </div>
            <button
              type="submit"
              disabled={pending}
              className="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-800 hover:bg-slate-50 disabled:opacity-60"
            >
              Save destination &amp; domains
            </button>
          </form>
          <form className="mt-6 space-y-3 border-t border-slate-100 pt-6" onSubmit={onSaveRuntimeVolumes}>
            <h3 className="text-sm font-semibold text-slate-800">Persistent volume mounts</h3>
            <p className="text-xs text-slate-500">
              Each entry maps a host directory (derived from <code className="rounded bg-slate-100 px-1">name</code> under
              the worker&apos;s <code className="rounded bg-slate-100 px-1">DEPLOYWERK_VOLUMES_ROOT</code>) into the
              container at <code className="rounded bg-slate-100 px-1">container_path</code>. JSON array; use{" "}
              <code className="rounded bg-slate-100 px-1">[]</code> to clear.
            </p>
            <div>
              <label className="text-xs font-medium text-slate-500" htmlFor="rv">
                Runtime volumes (JSON)
              </label>
              <textarea
                id="rv"
                value={runtimeVolumesJson}
                onChange={(e) => setRuntimeVolumesJson(e.target.value)}
                rows={5}
                className="mt-1 w-full max-w-2xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-xs"
                placeholder='[{"name":"data","container_path":"/data"}]'
              />
            </div>
            <button
              type="submit"
              disabled={pending}
              className="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-800 hover:bg-slate-50 disabled:opacity-60"
            >
              Save volume mounts
            </button>
          </form>
          <form className="mt-6 space-y-3 border-t border-slate-100 pt-6" onSubmit={onSaveDeployPolicy}>
            <h3 className="text-sm font-semibold text-slate-800">Deploy policy</h3>
            <p className="text-xs text-slate-500">
              Manual deploys from the UI can require owner/admin approval. Git webhooks enqueue immediately.
            </p>
            {deployStrategyEdit !== "standard" && (
              <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950">
                <strong className="font-semibold">Strategy is metadata today.</strong> The worker still performs a
                single-container replace (same as <code className="rounded bg-amber-100/80 px-1">standard</code>).
                Blue/green, canary, and rolling do not yet switch traffic or run side-by-side containers.
                {requireDeployApprovalEdit && (
                  <>
                    {" "}
                    Manual deploys wait on the{" "}
                    <Link
                      to={`/app/teams/${teamId}/deployments`}
                      className="font-medium text-amber-900 underline-offset-2 hover:underline"
                    >
                      Deployments
                    </Link>{" "}
                    page for approval.
                  </>
                )}
              </div>
            )}
            <div>
              <label className="text-xs font-medium text-slate-500">Strategy (worker path)</label>
              <select
                value={deployStrategyEdit}
                onChange={(e) => setDeployStrategyEdit(e.target.value)}
                className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
              >
                <option value="standard">standard</option>
                <option value="blue_green">blue_green</option>
                <option value="canary">canary</option>
                <option value="rolling">rolling</option>
              </select>
            </div>
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                checked={requireDeployApprovalEdit}
                onChange={(e) => setRequireDeployApprovalEdit(e.target.checked)}
              />
              Require approval for manual deploys
            </label>
            <div>
              <label className="text-xs font-medium text-slate-500" htmlFor="prehook">
                Pre-deploy hook (HTTPS POST, optional)
              </label>
              <input
                id="prehook"
                value={preDeployHookEdit}
                onChange={(e) => setPreDeployHookEdit(e.target.value)}
                placeholder="https://example.com/hooks/pre"
                className="mt-1 w-full max-w-2xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              />
            </div>
            <div>
              <label className="text-xs font-medium text-slate-500" htmlFor="posthook">
                Post-deploy hook (HTTPS POST, optional)
              </label>
              <input
                id="posthook"
                value={postDeployHookEdit}
                onChange={(e) => setPostDeployHookEdit(e.target.value)}
                placeholder="https://example.com/hooks/post"
                className="mt-1 w-full max-w-2xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              />
              <p className="mt-1 text-xs text-slate-500">
                Worker POSTs JSON <code className="rounded bg-slate-100 px-1">phase</code>,{" "}
                <code className="rounded bg-slate-100 px-1">job_id</code>,{" "}
                <code className="rounded bg-slate-100 px-1">application_id</code>, slug, and image. Non-2xx fails the
                job (post-deploy runs after the container starts).
              </p>
            </div>
            <button
              type="submit"
              disabled={pending}
              className="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-800 hover:bg-slate-50 disabled:opacity-60"
            >
              Save deploy policy
            </button>
          </form>
          <form className="mt-6 space-y-3 border-t border-slate-100 pt-6" onSubmit={onSaveGit}>
            <h3 className="text-sm font-semibold text-slate-800">Git push → deploy</h3>
            <p className="text-xs text-slate-500">
              Create one application per branch rule (e.g. production on <code className="rounded bg-slate-100 px-1">main</code>
              , previews on <code className="rounded bg-slate-100 px-1">*</code> or{" "}
              <code className="rounded bg-slate-100 px-1">feature/*</code>). Configure the webhook under{" "}
              <strong>Sandboxes</strong>.
            </p>
            <div>
              <label className="text-xs font-medium text-slate-500">Git repo URL</label>
              <input
                value={gitUrlEdit}
                onChange={(e) => setGitUrlEdit(e.target.value)}
                className="mt-1 w-full max-w-2xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              />
            </div>
            <div>
              <label className="text-xs font-medium text-slate-500">
                Repo path / <code className="text-slate-600">owner/repo</code> (GitHub) or GitLab{" "}
                <code className="text-slate-600">group/project</code>
              </label>
              <input
                value={gitFullNameEdit}
                onChange={(e) => setGitFullNameEdit(e.target.value)}
                placeholder="octocat/Hello-World"
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              />
            </div>
            <div>
              <label className="text-xs font-medium text-slate-500">Branch pattern</label>
              <input
                value={branchPatternEdit}
                onChange={(e) => setBranchPatternEdit(e.target.value)}
                placeholder="main, *, or release/*"
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              />
            </div>
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                checked={autoDeployEdit}
                onChange={(e) => setAutoDeployEdit(e.target.checked)}
              />
              Auto-deploy on Git push (webhook)
            </label>
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                checked={prPreviewEdit}
                onChange={(e) => setPrPreviewEdit(e.target.checked)}
              />
              PR preview deploys (GitHub App — register installation under Integrations)
            </label>
            <h3 className="pt-4 text-sm font-semibold text-slate-800">Build image on server (optional)</h3>
            <p className="text-xs text-slate-500">
              When enabled, deploy clones <strong>Git repo URL</strong> on the destination host, runs{" "}
              <code className="rounded bg-slate-100 px-1">docker build</code>, then runs the new image. Requires{" "}
              <code className="rounded bg-slate-100 px-1">git</code> and Docker on the server. Use a public clone URL
              unless the host already has credentials.
            </p>
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                checked={buildFromGitEdit}
                onChange={(e) => setBuildFromGitEdit(e.target.checked)}
              />
              Build image from Git on deploy (instead of docker pull)
            </label>
            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <label className="text-xs font-medium text-slate-500">Git ref for clone (branch/tag)</label>
                <input
                  value={gitBuildRefEdit}
                  onChange={(e) => setGitBuildRefEdit(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                />
              </div>
              <div>
                <label className="text-xs font-medium text-slate-500">Dockerfile path (repo root relative)</label>
                <input
                  value={dockerfilePathEdit}
                  onChange={(e) => setDockerfilePathEdit(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                />
              </div>
            </div>
            <button
              type="submit"
              disabled={pending}
              className="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-800 hover:bg-slate-50 disabled:opacity-60"
            >
              Save Git settings
            </button>
          </form>
          <form className="mt-4 space-y-3" onSubmit={onSaveEnv}>
            <div>
              <label className="text-xs font-medium text-slate-500" htmlFor="ev">
                Env vars (JSON)
              </label>
              <textarea
                id="ev"
                value={envJson}
                onChange={(e) => setEnvJson(e.target.value)}
                rows={8}
                className="mt-1 w-full max-w-2xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-xs"
              />
            </div>
            <button
              type="submit"
              disabled={pending}
              className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700 disabled:opacity-60"
            >
              Save env vars
            </button>
          </form>
        </div>
      )}

      {canCreateApplication && (
        <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
          <h2 className="text-lg font-semibold text-slate-900">New application</h2>
          <form className="mt-4 space-y-4" onSubmit={onCreate}>
            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <label className="block text-sm font-medium text-slate-700" htmlFor="aname">
                  Name
                </label>
                <input
                  id="aname"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                  required
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-slate-700" htmlFor="aslg">
                  Slug (optional)
                </label>
                <input
                  id="aslg"
                  value={slug}
                  onChange={(e) => setSlug(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                />
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="img">
                Docker image
              </label>
              <input
                id="img"
                value={dockerImage}
                onChange={(e) => setDockerImage(e.target.value)}
                placeholder="nginx:alpine or local/build when using Git build"
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                required
              />
              <p className="mt-1 text-xs text-slate-500">
                For Git build mode, this image is ignored for pull; use a placeholder like{" "}
                <code className="rounded bg-slate-100 px-1">local/build</code>.
              </p>
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="dest">
                Destination (for deploy)
              </label>
              <select
                id="dest"
                value={destinationId}
                onChange={(e) => setDestinationId(e.target.value)}
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 text-sm"
              >
                <option value="">None</option>
                {destinations.map((d) => (
                  <option key={d.id} value={d.id}>
                    {d.name} ({d.slug})
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="dom">
                Domains (comma or newline, optional)
              </label>
              <textarea
                id="dom"
                value={domainsStr}
                onChange={(e) => setDomainsStr(e.target.value)}
                rows={2}
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
              {bootstrap?.apps_base_domain && (
                <p className="mt-1 text-xs text-slate-500">
                  Leave empty to get a random hostname under{" "}
                  <span className="font-mono">*.{bootstrap.apps_base_domain}</span>.
                </p>
              )}
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="git">
                Git repo URL (optional)
              </label>
              <input
                id="git"
                value={gitUrl}
                onChange={(e) => setGitUrl(e.target.value)}
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="gfn">
                Repo path (GitHub <code className="text-slate-600">owner/repo</code> or GitLab{" "}
                <code className="text-slate-600">group/project</code>; optional if URL is enough)
              </label>
              <input
                id="gfn"
                value={gitFullName}
                onChange={(e) => setGitFullName(e.target.value)}
                placeholder="octocat/Hello-World or mygroup/myproject"
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="gbp">
                Branch pattern for webhook
              </label>
              <input
                id="gbp"
                value={branchPattern}
                onChange={(e) => setBranchPattern(e.target.value)}
                className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              />
            </div>
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                checked={autoDeployPush}
                onChange={(e) => setAutoDeployPush(e.target.checked)}
              />
              Enable auto-deploy on Git push
            </label>
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                checked={prPreviewPush}
                onChange={(e) => setPrPreviewPush(e.target.checked)}
              />
              Enable PR preview deploys (GitHub App)
            </label>
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                checked={buildFromGit}
                onChange={(e) => setBuildFromGit(e.target.checked)}
              />
              Build image from Git on deploy (server runs git clone + docker build)
            </label>
            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <label className="block text-sm font-medium text-slate-700" htmlFor="gbr">
                  Git ref for clone
                </label>
                <input
                  id="gbr"
                  value={gitBuildRef}
                  onChange={(e) => setGitBuildRef(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-slate-700" htmlFor="dfp">
                  Dockerfile path
                </label>
                <input
                  id="dfp"
                  value={dockerfilePath}
                  onChange={(e) => setDockerfilePath(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                />
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="rvc">
                Persistent volume mounts (JSON, optional)
              </label>
              <textarea
                id="rvc"
                value={runtimeVolumesCreateJson}
                onChange={(e) => setRuntimeVolumesCreateJson(e.target.value)}
                rows={3}
                className="mt-1 w-full max-w-2xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-xs"
                placeholder='[{"name":"data","container_path":"/data"}]'
              />
            </div>
            <button
              type="submit"
              disabled={pending}
              className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700 disabled:opacity-60"
            >
              {pending ? "Creating…" : "Create"}
            </button>
          </form>
        </div>
      )}
    </div>
  );
}
