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

const stdin = createInterface({
  input: process.stdin,
  crlfDelay: Infinity
});

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

  const displayName = String(
    envelope.request.payload.payload.displayName ?? ""
  );

  if (displayName === "transport-error") {
    writeTransportError(envelope.requestId, "invalidEnvelope", "fixture transport error");
    return;
  }

  if (displayName === "unknown-remote-transport-code") {
    writeTransportError(envelope.requestId, "futureRemoteCode", "fixture unknown code");
    return;
  }

  if (displayName === "malformed") {
    process.stdout.write("not-json\n");
    return;
  }

  if (displayName === "unknown-response-request-id") {
    writeEnvelope({
      type: "commandOutcome",
      requestId: "missing-request",
      outcome: {
        type: "success",
        payload: {
          reply: {
            type: "playlistWrite",
            payload: {
              type: "createPlaylist",
              payload: {
                playlistId: "13"
              }
            }
          }
        }
      }
    });
    return;
  }

  if (displayName === "blank") {
    process.stdout.write("\n");
    writeSuccess(envelope, "10");
    return;
  }

  if (displayName === "stderr") {
    process.stderr.write("fixture diagnostic\n");
    writeSuccess(envelope, "11");
    return;
  }

  if (displayName === "exit-pending") {
    process.exit(7);
  }

  if (displayName === "protocol-error") {
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

  if (displayName === "delayed") {
    globalThis.setTimeout(() => writeSuccess(envelope, "12"), 50);
    return;
  }

  if (displayName.startsWith("concurrent-")) {
    concurrentRequests.push(envelope);
    if (concurrentRequests.length === 2) {
      const [first, second] = concurrentRequests.splice(0, 2);
      writeSuccess(second, "2");
      writeSuccess(first, "1");
    }
    return;
  }

  const bulkMatch = /^bulk-(\d+)$/.exec(displayName);
  if (bulkMatch !== null) {
    writeSuccess(envelope, bulkMatch[1] ?? "99");
    return;
  }

  writeSuccess(envelope, "1");
});

stdin.on("close", () => {
  process.exit(0);
});

function writeSuccess(envelope: RequestEnvelope, playlistId: string): void {
  writeEnvelope({
    type: "commandOutcome",
    requestId: envelope.requestId,
    outcome: {
      type: "success",
      payload: {
        reply: {
          type: "playlistWrite",
          payload: {
            type: "createPlaylist",
            payload: {
              playlistId
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
