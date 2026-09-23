#!/usr/bin/env node
// Rust half of `npm run verify`: clippy (warnings are errors) then the test suite.
//
// The only reason to skip is a Linux machine without the GTK/WebKit system libraries
// Tauri links against — there the crate cannot compile at all. A clippy or test
// failure is never a reason to skip: it fails the run like any other check.
import { spawnSync } from "node:child_process";

const MANIFEST = "src-tauri/Cargo.toml";

function gtkMissing() {
  if (process.platform !== "linux") return false;
  const probe = spawnSync("pkg-config", ["--exists", "gdk-3.0"], { stdio: "ignore" });
  // A missing pkg-config (probe.error) means we cannot know, and so does a non-zero exit.
  return probe.error !== undefined || probe.status !== 0;
}

function run(args) {
  const result = spawnSync("cargo", args, { stdio: "inherit" });
  if (result.error) {
    console.error(`verify:rust: could not run cargo: ${result.error.message}`);
    process.exit(1);
  }
  if (result.status !== 0) process.exit(result.status ?? 1);
}

if (gtkMissing()) {
  console.log(
    "Skipping Rust checks (GTK system libs not found – run in an environment with Tauri prerequisites installed)",
  );
  process.exit(0);
}

run(["clippy", "--manifest-path", MANIFEST, "--all-targets", "--", "-D", "warnings"]);
run(["test", "--manifest-path", MANIFEST]);
