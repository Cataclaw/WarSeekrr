// npm run release -- 0.1.1
import { execSync } from "node:child_process";
import { copyFileSync, mkdirSync, readdirSync, rmSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error("usage: npm run release -- <version>   e.g. 0.1.1");
  process.exit(1);
}
const tag = `v${version}`;
const run = (cmd, opts = {}) => execSync(cmd, { stdio: "inherit", ...opts });
const out = (cmd) => execSync(cmd, { encoding: "utf8" }).trim();

if (out("git status --porcelain")) throw new Error("working tree not clean");
run("git fetch --tags -q");
if (out(`git tag -l ${tag}`)) throw new Error(`${tag} already exists`);

run("cargo test --locked -q", { cwd: "src-tauri" });
run(`npm run tauri build -- --bundles nsis --config "{\\"version\\":\\"${version}\\"}"`);

rmSync("dist", { recursive: true, force: true });
mkdirSync("dist");
const nsis = "src-tauri/target/release/bundle/nsis";
const setup = readdirSync(nsis).find((f) => f.includes(`_${version}_`) && f.endsWith("-setup.exe"));
if (!setup) throw new Error(`no ${version} installer in ${nsis}`);
copyFileSync(`${nsis}/${setup}`, `dist/WarSeekrr-${version}-setup.exe`);
copyFileSync("src-tauri/target/release/warseekrr.exe", `dist/WarSeekrr-${version}-portable.exe`);

run(`git tag ${tag}`);
run(`git push -q origin HEAD ${tag}`);
run(
  `gh release create ${tag} dist/WarSeekrr-${version}-setup.exe dist/WarSeekrr-${version}-portable.exe --title "WarSeekrr ${version}" --generate-notes`,
);
