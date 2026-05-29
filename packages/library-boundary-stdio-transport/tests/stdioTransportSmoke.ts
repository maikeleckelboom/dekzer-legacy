import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import {
  createLibraryBoundaryClient,
  type LibraryBoundaryClient
} from "@dekzer/library-boundary-client";

import {
  LibraryBoundaryStdioTransport
} from "../src/index.js";

const binaryPath =
  process.env.DEKZER_LIBRARY_BOUNDARY_STDIO_BINARY ??
  resolve(
    process.cwd(),
    "..",
    "..",
    "target",
    "debug",
    process.platform === "win32"
      ? "library-boundary-stdio.exe"
      : "library-boundary-stdio"
  );

must(
  existsSync(binaryPath),
  `expected Rust stdio binary to exist at ${binaryPath}`
);

const tempRoot = await mkdtemp(join(tmpdir(), "dekzer-stdio-smoke-"));
const userDataPath = join(tempRoot, "user-data");
const sourceRoot = join(tempRoot, "source-root");
const crateDirectory = join(sourceRoot, "Crate");
await mkdir(crateDirectory, { recursive: true });
await writeFile(join(crateDirectory, "amen.wav"), Buffer.from("not-real-audio"));

try {
  const first = await openClient();
  const registered = await first.client.registerLocalRoot({
    absolutePath: sourceRoot
  });
  must(/^[1-9]\d*$/.test(registered.rootId), "rootId is a positive string id");

  const scan = await first.client.startRootScan({
    rootId: registered.rootId
  });
  must(/^[1-9]\d*$/.test(scan.scanRunId), "scanRunId is a positive string id");

  let scanCompleted = false;
  let cursor: number | null = null;
  for (let i = 0; i < 30; i++) {
    const eventsReply = await first.client.readAfterBoundaryEvents({
      lastSeenEventSequence: cursor,
      maxEvents: 32
    });
    cursor = eventsReply.latestEventSequence;
    scanCompleted = eventsReply.events.some(
      (e) =>
        e.type === "sourceScanEvent" &&
        e.payload.kind === "sourceScanCompleted" &&
        e.payload.rootId === registered.rootId
    );
    if (scanCompleted) break;
    await new Promise((r) => setTimeout(r, 100));
  }
  must(scanCompleted, "scan completes on a tiny directory");

  const rootWindow = await first.client.readLibraryTreeChildren({
    entryPoint: {
      type: "source",
      payload: {
        sourceId: registered.rootId
      }
    },
    parentSourceDirectoryId: null,
    offset: 0,
    limit: 10
  });
  const crateRow = rootWindow.window?.rows.find(
    (row) => row.displayName === "Crate"
  );
  must(crateRow !== undefined, "scanned folder is visible in hierarchy");
  equal(crateRow.nodeKind, "directory", "folder row is a directory");
  must(
    crateRow.sourceDirectoryId !== null,
    "directory row carries durable sourceDirectoryId"
  );

  const fileWindow = await first.client.readLibraryTreeChildren({
    entryPoint: {
      type: "source",
      payload: {
        sourceId: registered.rootId
      }
    },
    parentSourceDirectoryId: crateRow.sourceDirectoryId,
    offset: 0,
    limit: 10
  });
  equal(
    fileWindow.window?.rows[0]?.displayName,
    "amen.wav",
    "nested file appears after scan"
  );
  await first.transport.close();

  const reopened = await openClient();
  const reopenedWindow = await reopened.client.readLibraryTreeChildren({
    entryPoint: {
      type: "source",
      payload: {
        sourceId: registered.rootId
      }
    },
    parentSourceDirectoryId: crateRow.sourceDirectoryId,
    offset: 0,
    limit: 10
  });
  equal(
    reopenedWindow.window?.rows[0]?.displayName,
    "amen.wav",
    "persisted hierarchy survives a new stdio process"
  );
  await reopened.transport.close();
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}

async function openClient(): Promise<{
  readonly transport: LibraryBoundaryStdioTransport;
  readonly client: LibraryBoundaryClient;
}> {
  const transport = new LibraryBoundaryStdioTransport({
    serverBinaryPath: binaryPath,
    userDataPath,
    environment: "development",
    diagnostics: (diagnostic) => {
      if (diagnostic.stream === "stderr") {
        throw new Error(`unexpected stdio server stderr: ${diagnostic.line}`);
      }
    }
  });
  await transport.ready;
  return {
    transport,
    client: createLibraryBoundaryClient(transport)
  };
}

function must(condition: unknown, message: string): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}

function equal<T>(actual: T, expected: T, message: string): void {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${String(expected)}, got ${String(actual)}`);
  }
}
