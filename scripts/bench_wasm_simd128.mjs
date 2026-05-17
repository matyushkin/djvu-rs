#!/usr/bin/env node
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { performance } from "node:perf_hooks";

const require = createRequire(import.meta.url);

function usage() {
  console.error(`Usage:
  node scripts/bench_wasm_simd128.mjs \\
    --scalar target/wasm-bench/scalar \\
    --simd target/wasm-bench/simd128 \\
    [--fixture tests/fixtures/boy.djvu] \\
    [--iterations 50] [--warmup 10] [--dpi 150] [--json]
`);
}

function parseArgs(argv) {
  const args = {
    fixture: "tests/fixtures/boy.djvu",
    iterations: 50,
    warmup: 10,
    dpi: 150,
    json: false,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--json") {
      args.json = true;
      continue;
    }
    const next = argv[i + 1];
    if (next === undefined) {
      throw new Error(`missing value for ${arg}`);
    }
    i += 1;
    switch (arg) {
      case "--scalar":
        args.scalar = next;
        break;
      case "--simd":
        args.simd = next;
        break;
      case "--fixture":
        args.fixture = next;
        break;
      case "--iterations":
        args.iterations = Number.parseInt(next, 10);
        break;
      case "--warmup":
        args.warmup = Number.parseInt(next, 10);
        break;
      case "--dpi":
        args.dpi = Number.parseInt(next, 10);
        break;
      default:
        throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (!args.scalar || !args.simd) {
    throw new Error("--scalar and --simd are required");
  }
  for (const key of ["iterations", "warmup", "dpi"]) {
    if (!Number.isInteger(args[key]) || args[key] <= 0) {
      throw new Error(`--${key} must be a positive integer`);
    }
  }
  return args;
}

function loadPkg(dir) {
  return require(resolve(dir, "djvu_rs.js"));
}

function percentile(sorted, p) {
  const idx = Math.min(sorted.length - 1, Math.floor((sorted.length - 1) * p));
  return sorted[idx];
}

function summarize(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  const sum = samples.reduce((acc, v) => acc + v, 0);
  return {
    min_ms: sorted[0],
    median_ms: percentile(sorted, 0.5),
    p90_ms: percentile(sorted, 0.9),
    mean_ms: sum / samples.length,
  };
}

function timeIt(fn) {
  const t0 = performance.now();
  const checksum = fn();
  return { ms: performance.now() - t0, checksum };
}

function benchPackage(label, pkg, bytes, args) {
  const { WasmDocument } = pkg;

  const benches = [
    {
      name: "parse_document",
      fn: () => WasmDocument.from_bytes(bytes).page_count(),
    },
    {
      name: `render_${args.dpi}dpi_fresh_doc`,
      fn: () => {
        const doc = WasmDocument.from_bytes(bytes);
        const page = doc.page(0);
        const pixels = page.render(args.dpi);
        return pixelChecksum(pixels) ^ page.width_at(args.dpi) ^ page.height_at(args.dpi);
      },
    },
    {
      name: `render_${args.dpi}dpi_cached_page`,
      setup: () => {
        const doc = WasmDocument.from_bytes(bytes);
        return doc.page(0);
      },
      fn: (page) => {
        const pixels = page.render(args.dpi);
        return pixelChecksum(pixels) ^ page.width_at(args.dpi) ^ page.height_at(args.dpi);
      },
    },
    {
      name: `progressive_${args.dpi}dpi_chunk0`,
      setup: () => {
        const doc = WasmDocument.from_bytes(bytes);
        return doc.page(0);
      },
      fn: (page) => {
        const pixels = page.render_progressive(args.dpi, 0);
        return pixelChecksum(pixels) ^ page.bg44_chunk_count();
      },
    },
  ];

  const results = [];
  for (const bench of benches) {
    const state = bench.setup ? bench.setup() : undefined;
    for (let i = 0; i < args.warmup; i += 1) {
      bench.fn(state);
    }
    const samples = [];
    let expectedChecksum;
    for (let i = 0; i < args.iterations; i += 1) {
      const { ms, checksum } = timeIt(() => bench.fn(state));
      if (expectedChecksum === undefined) {
        expectedChecksum = checksum;
      } else if (checksum !== expectedChecksum) {
        throw new Error(
          `${label}/${bench.name} produced unstable checksum: ${checksum} != ${expectedChecksum}`,
        );
      }
      samples.push(ms);
    }
    results.push({
      package: label,
      benchmark: bench.name,
      iterations: args.iterations,
      checksum: expectedChecksum,
      ...summarize(samples),
    });
  }
  return results;
}

function pixelChecksum(pixels) {
  let hash = 0x811c9dc5;
  for (let i = 0; i < pixels.length; i += 1) {
    hash ^= pixels[i];
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  hash ^= pixels.length;
  return hash >>> 0;
}

function pctDelta(scalar, simd) {
  return ((simd - scalar) / scalar) * 100;
}

function printMarkdown(results) {
  const byBench = new Map();
  for (const row of results) {
    if (!byBench.has(row.benchmark)) {
      byBench.set(row.benchmark, {});
    }
    byBench.get(row.benchmark)[row.package] = row;
  }
  console.log("| Benchmark | scalar median ms | simd128 median ms | delta |");
  console.log("|-----------|-----------------:|------------------:|------:|");
  for (const [name, rows] of byBench) {
    const scalar = rows.scalar?.median_ms;
    const simd = rows.simd128?.median_ms;
    const delta = scalar === undefined || simd === undefined ? NaN : pctDelta(scalar, simd);
    console.log(
      `| \`${name}\` | ${scalar.toFixed(3)} | ${simd.toFixed(3)} | ${delta.toFixed(1)}% |`,
    );
  }
}

try {
  const args = parseArgs(process.argv.slice(2));
  const bytes = readFileSync(args.fixture);
  const scalar = loadPkg(args.scalar);
  const simd = loadPkg(args.simd);
  const results = [
    ...benchPackage("scalar", scalar, bytes, args),
    ...benchPackage("simd128", simd, bytes, args),
  ];
  assertMatchingChecksums(results);
  if (args.json) {
    console.log(JSON.stringify({ args, node: process.version, results }, null, 2));
  } else {
    printMarkdown(results);
  }
} catch (err) {
  usage();
  console.error(err instanceof Error ? err.message : err);
  process.exit(2);
}

function assertMatchingChecksums(results) {
  const byBench = new Map();
  for (const row of results) {
    if (!byBench.has(row.benchmark)) {
      byBench.set(row.benchmark, {});
    }
    byBench.get(row.benchmark)[row.package] = row.checksum;
  }
  for (const [name, rows] of byBench) {
    if (rows.scalar !== rows.simd128) {
      throw new Error(
        `${name} checksum mismatch between scalar (${rows.scalar}) and simd128 (${rows.simd128})`,
      );
    }
  }
}
