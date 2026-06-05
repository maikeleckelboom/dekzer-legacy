import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import {
  createLibraryBoundaryClient,
  type LibraryBoundaryClient
} from "@dekzer/library-boundary-client";
import type {
  ReadLibraryBoundaryEventsAfterReply
} from "@dekzer/library-boundary-contract";

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
const durableStorePath = join(userDataPath, "development", "library.sqlite3");
const commandTimeoutMs = 10_000;
const closeTimeoutMs = 5_000;
await mkdir(crateDirectory, { recursive: true });
await writeFile(join(crateDirectory, "amen.wav"), Buffer.from("not-real-audio"));

try {
  let registeredRootId = "";
  let crateSourceDirectoryId: string | null = null;

  await withClient("initial stdio smoke process", async (first) => {
    const registered = await withTimeout(
      first.client.registerLocalRoot({
        absolutePath: sourceRoot
      }),
      "registerLocalRoot",
      commandTimeoutMs
    );
    registeredRootId = registered.rootId;
    must(/^[1-9]\d*$/.test(registeredRootId), "rootId is a positive string id");

    const scan = await withTimeout(
      first.client.startRootScan({
        rootId: registeredRootId
      }),
      "startRootScan",
      commandTimeoutMs
    );
    must(/^[1-9]\d*$/.test(scan.scanRunId), "scanRunId is a positive string id");

    let scanCompleted = false;
    let cursor: number | null = null;
    for (let i = 0; i < 30; i++) {
      const eventsReply: ReadLibraryBoundaryEventsAfterReply = await withTimeout(
        first.client.readAfterBoundaryEvents({
          lastSeenEventSequence: cursor,
          maxEvents: 32
        }),
        "readAfterBoundaryEvents",
        commandTimeoutMs
      );
      cursor = eventsReply.latestEventSequence;
      scanCompleted = eventsReply.events.some(
        (e) =>
          e.type === "sourceScanEvent" &&
          e.payload.kind === "sourceScanCompleted" &&
          e.payload.rootId === registeredRootId
      );
      if (scanCompleted) break;
      await sleep(100);
    }
    must(scanCompleted, "scan completes on a tiny directory");

    const rootWindow = await withTimeout(
      first.client.readLibraryTreeChildren({
        entryPoint: {
          type: "source",
          payload: {
            sourceId: registeredRootId
          }
        },
        parentSourceDirectoryId: null,
        offset: 0,
        limit: 10
      }),
      "readLibraryTreeChildren root",
      commandTimeoutMs
    );
    const crateRow = rootWindow.window?.rows.find(
      (row) => row.displayName === "Crate"
    );
    must(crateRow !== undefined, "scanned folder is visible in hierarchy");
    equal(crateRow.nodeKind, "directory", "folder row is a directory");
    must(
      crateRow.sourceDirectoryId !== null,
      "directory row carries durable sourceDirectoryId"
    );
    crateSourceDirectoryId = crateRow.sourceDirectoryId;

    const crateWindow = await waitForNestedDirectory(first.client, {
      sourceId: registeredRootId,
      parentSourceDirectoryId: crateSourceDirectoryId,
      label: "readLibraryTreeChildren nested"
    });
    equal(
      crateWindow.window?.totalRows,
      0,
      "nested directory child window is navigation-only"
    );

    const crateContents = await withTimeout(
      first.client.readContents({
        scope: {
          type: "directory",
          payload: {
            sourceId: registeredRootId,
            sourceDirectoryId: crateSourceDirectoryId!
          }
        },
        policy: {
          mediaClasses: ["audio"],
          rowProfile: { kind: "sourceFile" }
        },
        recursion: "immediate",
        limit: 10
      }),
      "readContents Crate",
      commandTimeoutMs
    );
    must(
      crateContents.result.state === "ready",
      "contents read for Crate directory is ready"
    );
    must(
      crateContents.result.rows.some((row) => row.fileName === "amen.wav"),
      "nested audio file appears through contents read"
    );
  });

  await withClient("reopened stdio smoke process", async (reopened) => {
    const reopenedWindow = await waitForNestedDirectory(reopened.client, {
      sourceId: registeredRootId,
      parentSourceDirectoryId: crateSourceDirectoryId,
      label: "readLibraryTreeChildren reopened"
    });
    equal(
      reopenedWindow.window?.totalRows,
      0,
      "nested directory child window is navigation-only after reopen"
    );

    const reopenedContents = await withTimeout(
      reopened.client.readContents({
        scope: {
          type: "directory",
          payload: {
            sourceId: registeredRootId,
            sourceDirectoryId: crateSourceDirectoryId!
          }
        },
        policy: {
          mediaClasses: ["audio"],
          rowProfile: { kind: "sourceFile" }
        },
        recursion: "immediate",
        limit: 10
      }),
      "readContents reopened Crate",
      commandTimeoutMs
    );
    must(
      reopenedContents.result.state === "ready",
      "contents read persists across a reopened stdio process: state is ready"
    );
    must(
      reopenedContents.result.rows.some((row) => row.fileName === "amen.wav"),
      "nested audio file appears through contents read after reopen"
    );
  });
} finally {
  await removeTempRoot();
}

async function waitForNestedDirectory(
  client: LibraryBoundaryClient,
  input: {
    readonly sourceId: string;
    readonly parentSourceDirectoryId: string | null;
    readonly label: string;
  }
): Promise<Awaited<ReturnType<LibraryBoundaryClient["readLibraryTreeChildren"]>>> {
  for (let i = 0; i < 30; i++) {
    const crateWindow = await withTimeout(
      client.readLibraryTreeChildren({
        entryPoint: {
          type: "source",
          payload: {
            sourceId: input.sourceId
          }
        },
        parentSourceDirectoryId: input.parentSourceDirectoryId,
        offset: 0,
        limit: 10
      }),
      input.label,
      commandTimeoutMs
    );
    if (crateWindow.window?.totalRows === 0) {
      return crateWindow;
    }
    await sleep(100);
  }

  throw new Error(`${input.label} did not settle to an empty navigation-only child window`);
}

async function withClient<T>(
  label: string,
  action: (opened: {
    readonly transport: LibraryBoundaryStdioTransport;
    readonly client: LibraryBoundaryClient;
  }) => Promise<T>
): Promise<T> {
  const opened = await openClient(label);
  let actionError: unknown = null;
  try {
    return await action(opened);
  } catch (error) {
    actionError = error;
    throw error;
  } finally {
    try {
      await withTimeout(
        opened.transport.close(),
        `${label} transport close`,
        closeTimeoutMs * 2 + 1_000
      );
    } catch (closeError) {
      const detail = actionError === null
        ? formatError(closeError)
        : `action failed with ${formatError(actionError)}; cleanup failed with ${formatError(closeError)}`;
      throw new Error(`${label} cleanup failed: ${detail}`);
    }
  }
}

async function removeTempRoot(): Promise<void> {
  const retryDelays = [25, 50, 100, 200, 400];
  let lastError: unknown = null;

  for (let attempt = 0; attempt <= retryDelays.length; attempt++) {
    try {
      await rm(tempRoot, { recursive: true, force: true });
      return;
    } catch (error) {
      lastError = error;
      if (!isRetryableCleanupError(error) || attempt === retryDelays.length) {
        break;
      }
      await sleep(retryDelays[attempt] ?? 0);
    }
  }

  throw new Error(
    `failed to remove stdio smoke temp directory ${tempRoot}; likely locked SQLite path ${durableStorePath}; cleanup error: ${formatError(lastError)}`
  );
}

async function openClient(label: string): Promise<{
  readonly transport: LibraryBoundaryStdioTransport;
  readonly client: LibraryBoundaryClient;
}> {
  const transport = new LibraryBoundaryStdioTransport({
    serverBinaryPath: binaryPath,
    userDataPath,
    environment: "development",
    closeTimeoutMs,
    diagnostics: (diagnostic) => {
      if (diagnostic.stream === "stderr") {
        throw new Error(`unexpected stdio server stderr during ${label}: ${diagnostic.line}`);
      }
    }
  });
  await withTimeout(transport.ready, `${label} ready`, commandTimeoutMs);
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

function withTimeout<T>(
  promise: Promise<T>,
  label: string,
  timeoutMs: number
): Promise<T> {
  let timeoutId: NodeJS.Timeout | null = null;
  const timeout = new Promise<never>((_, reject) => {
    timeoutId = globalThis.setTimeout(() => {
      reject(new Error(`${label} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
  });

  return Promise.race([promise, timeout]).finally(() => {
    if (timeoutId !== null) {
      globalThis.clearTimeout(timeoutId);
    }
  });
}

function sleep(milliseconds: number): Promise<void> {
  return new Promise((resolve) => {
    globalThis.setTimeout(resolve, milliseconds);
  });
}

function isRetryableCleanupError(error: unknown): boolean {
  return error instanceof Error &&
    "code" in error &&
    (error.code === "EBUSY" ||
      error.code === "EPERM" ||
      error.code === "ENOTEMPTY");
}

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return `${error.name}: ${error.message}`;
  }

  return String(error);
}
