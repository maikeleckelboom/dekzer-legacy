import { createInterface } from "node:readline";

type RequestEnvelope = {
  readonly type: "command";
  readonly requestId: string;
  readonly request: {
    readonly type: string;
    readonly payload: {
      readonly type: string;
      readonly payload: Record<string, unknown>;
    };
  };
};

const concurrentRequests: RequestEnvelope[] = [];
const startupMode = process.argv[2] ?? "ready";

const stdin = createInterface({
  input: process.stdin,
  crlfDelay: Infinity
});

if (startupMode === "exit-before-ready") {
  process.exit(8);
}

if (startupMode === "malformed-before-ready") {
  process.stdout.write("not-json\n");
} else if (startupMode === "malformed-ready") {
  writeEnvelope({
    type: "ready",
    server: "wrongServer"
  });
} else if (startupMode === "delayed-ready") {
  globalThis.setTimeout(writeReady, 50);
} else {
  writeReady();
}

stdin.on("line", (line) => {
  if (line.trim().length === 0) {
    return;
  }

  let envelope: RequestEnvelope;
  try {
    envelope = JSON.parse(line) as RequestEnvelope;
  } catch {
    writeTransportError(null, "invalidFrame", "fixture received invalid JSON");
    return;
  }

  const commandFamily = envelope.request.type;
  const commandType = envelope.request.payload.type;

  if (commandFamily === "snapshotRead" && commandType === "readLocalBrowseEntryPoints") {
    writeLocalBrowseEntryPointsSuccess(envelope);
    return;
  }

  if (commandFamily === "snapshotRead" && commandType === "readLocalBrowseItems") {
    writeLocalBrowseItemsSuccess(envelope);
    return;
  }

  const requestedPath = String(
    envelope.request.payload.payload.requestedPath ?? ""
  );

  if (requestedPath === "transport-error") {
    writeTransportError(envelope.requestId, "invalidEnvelope", "fixture transport error");
    return;
  }

  if (requestedPath === "unknown-remote-transport-code") {
    writeTransportError(envelope.requestId, "futureRemoteCode", "fixture unknown code");
    return;
  }

  if (requestedPath === "malformed") {
    process.stdout.write("not-json\n");
    return;
  }

  if (requestedPath === "duplicate-ready") {
    writeReady();
    return;
  }

  if (requestedPath === "unknown-response-request-id") {
    writeEnvelope({
      type: "commandOutcome",
      requestId: "missing-request",
      outcome: {
        type: "success",
        payload: {
          reply: {
            type: "libraryRoots",
            payload: {
              type: "registerLocalRoot",
              payload: {
                type: "registered",
                payload: {
                  rootId: "13",
                  admittedRootPath: "fixture"
                }
              }
            }
          }
        }
      }
    });
    return;
  }

  if (requestedPath === "blank") {
    process.stdout.write("\n");
    writeSuccess(envelope, "10");
    return;
  }

  if (requestedPath === "stderr") {
    process.stderr.write("fixture diagnostic\n");
    writeSuccess(envelope, "11");
    return;
  }

  if (requestedPath === "exit-pending") {
    process.exit(7);
  }

  if (requestedPath === "protocol-error") {
    writeEnvelope({
      type: "commandOutcome",
      requestId: envelope.requestId,
      outcome: {
        type: "error",
        payload: {
          error: {
            type: "invalidRequest",
            payload: {
              detail: "fixture protocol error"
            }
          }
        }
      }
    });
    return;
  }

  if (requestedPath === "delayed") {
    globalThis.setTimeout(() => writeSuccess(envelope, "12"), 50);
    return;
  }

  if (requestedPath === "never-respond") {
    return;
  }

  if (requestedPath.startsWith("concurrent-")) {
    concurrentRequests.push(envelope);
    if (concurrentRequests.length === 2) {
      const [first, second] = concurrentRequests.splice(0, 2);
      writeSuccess(second, "2");
      writeSuccess(first, "1");
    }
    return;
  }

  const bulkMatch = /^bulk-(\d+)$/.exec(requestedPath);
  if (bulkMatch !== null) {
    writeSuccess(envelope, bulkMatch[1] ?? "99");
    return;
  }

  writeSuccess(envelope, "1");
});

stdin.on("close", () => {
  process.exit(0);
});

function writeSuccess(envelope: RequestEnvelope, rootId: string): void {
  writeEnvelope({
    type: "commandOutcome",
    requestId: envelope.requestId,
    outcome: {
      type: "success",
      payload: {
        reply: {
          type: "libraryRoots",
          payload: {
            type: "registerLocalRoot",
            payload: {
              type: "registered",
              payload: {
                rootId,
                admittedRootPath: `fixture:${rootId}`
              }
            }
          }
        }
      }
    }
  });
}

function writeLocalBrowseEntryPointsSuccess(envelope: RequestEnvelope): void {
  writeEnvelope({
    type: "commandOutcome",
    requestId: envelope.requestId,
    outcome: {
      type: "success",
      payload: {
        reply: {
          type: "snapshotRead",
          payload: {
            type: "localBrowseEntryPoints",
            payload: {
              status: "complete",
              entries: [
                {
                  identity: {
                    entryPointKind: "music",
                    resolvedPath: "C:\\Users\\DJ\\Music"
                  },
                  displayName: "Music",
                  status: "available",
                  platform: "windows",
                  availableOperations: [
                    { kind: "browseChildren" },
                    { kind: "chooseDescendant" },
                    {
                      kind: "requestSourceAdmission",
                      requestKind: "defaultMusicFolder",
                      resolvedPath: "C:\\Users\\DJ\\Music"
                    }
                  ],
                  failure: null
                }
              ],
              failure: null
            }
          }
        }
      }
    }
  });
}

function writeLocalBrowseItemsSuccess(envelope: RequestEnvelope): void {
  writeEnvelope({
    type: "commandOutcome",
    requestId: envelope.requestId,
    outcome: {
      type: "success",
      payload: {
        reply: {
          type: "snapshotRead",
          payload: {
            type: "localBrowseItems",
            payload: {
              status: "complete",
              windowIdentity: {
                entryPointKind: "music",
                resolvedRootPath: "C:\\Users\\DJ\\Music",
                resolvedParentPath: "C:\\Users\\DJ\\Music"
              },
              offset: 0,
              limit: 50,
              totalItems: 1,
              items: [
                {
                  identity: {
                    entryPointKind: "music",
                    resolvedRootPath: "C:\\Users\\DJ\\Music",
                    resolvedItemPath: "C:\\Users\\DJ\\Music\\Track.flac"
                  },
                  itemKind: "mediaFile",
                  displayName: "Track.flac",
                  status: "available",
                  platform: "windows",
                  fileKind: "audio",
                  mediaRelevance: "mediaRelevant",
                  availableOperations: [
                    {
                      kind: "requestSourceAdmission",
                      requestKind: "parentDirectory",
                      resolvedPath: "C:\\Users\\DJ\\Music"
                    }
                  ],
                  failure: null
                }
              ],
              failure: null
            }
          }
        }
      }
    }
  });
}

function writeTransportError(
  requestId: string | null,
  code: string,
  message: string
): void {
  writeEnvelope({
    type: "transportError",
    requestId,
    error: {
      code,
      message
    }
  });
}

function writeEnvelope(envelope: unknown): void {
  process.stdout.write(`${JSON.stringify(envelope)}\n`);
}

function writeReady(): void {
  writeEnvelope({
    type: "ready",
    server: "libraryBoundaryStdio"
  });
}
