import fs from "node:fs/promises";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const distAssets = path.join(root, "dist", "assets");

const maxEntryBytes = Number.parseInt(process.env.MAX_ENTRY_BYTES ?? "80000", 10);
const maxVendorReactBytes = Number.parseInt(
  process.env.MAX_VENDOR_REACT_BYTES ?? "350000",
  10,
);

async function main() {
  let files;
  try {
    files = await fs.readdir(distAssets);
  } catch {
    throw new Error(
      `dist/assets not found at ${distAssets}. Run \`npm run build\` first.`,
    );
  }

  const js = files.filter((f) => f.endsWith(".js"));
  const css = files.filter((f) => f.endsWith(".css"));
  if (js.length === 0 && css.length === 0) {
    throw new Error(`No build artifacts found in ${distAssets}.`);
  }

  const stats = await Promise.all(
    js.map(async (name) => {
      const p = path.join(distAssets, name);
      const st = await fs.stat(p);
      return { name, bytes: st.size };
    }),
  );

  const entry = stats.find((s) => s.name.startsWith("index-"));
  const vendorReact = stats.find((s) => s.name.startsWith("vendor-react-"));

  const problems = [];
  if (entry && entry.bytes > maxEntryBytes) {
    problems.push(
      `Entry chunk ${entry.name} is ${entry.bytes} bytes (max ${maxEntryBytes}).`,
    );
  }
  if (vendorReact && vendorReact.bytes > maxVendorReactBytes) {
    problems.push(
      `vendor-react chunk ${vendorReact.name} is ${vendorReact.bytes} bytes (max ${maxVendorReactBytes}).`,
    );
  }

  if (problems.length) {
    throw new Error(problems.join("\n"));
  }

  // Basic output for CI logs
  const top = stats.sort((a, b) => b.bytes - a.bytes).slice(0, 5);
  console.log("Bundle size check OK. Largest JS chunks:");
  for (const { name, bytes } of top) {
    console.log(`- ${name}: ${bytes} bytes`);
  }
}

main().catch((e) => {
  console.error(e instanceof Error ? e.message : String(e));
  process.exit(1);
});

