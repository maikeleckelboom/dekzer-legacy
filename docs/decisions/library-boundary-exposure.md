# Library Boundary Exposure

## Decision

Desktop library boundary operations have two exposure classes:

- **renderer-callable**: available to the renderer through the preload API and IPC. These operations must use
  renderer-safe inputs and may not let the renderer supply an absolute path for local root registration.
- **host-internal**: available inside the desktop host or library service path only. These operations may use host-owned
  values such as an absolute path after the desktop host has selected or validated them.

The trust split is: renderer-callable intent is not host-internal absolute-path registration. The renderer may request
"choose local root", but the desktop host owns the native folder picker and receives the absolute path. The library
service may then register that local root from the absolute path chosen by the desktop host.

Do not add compatibility aliases or alternate renderer paths for host-internal operations.

## Current Exposure

| Operation                | Exposure          | Owner / meaning                                                                                   |
| ------------------------ | ----------------- | ------------------------------------------------------------------------------------------------- |
| `chooseAndRegisterLocal` | renderer-callable | Desktop-host owned. Opens the native folder picker and has no `absolutePath` input.               |
| `registerLocalRoot`      | host-internal     | Library-service registration path. `absolutePath` is allowed only after native desktop selection. |
| `runScan`                | renderer-callable | Scan request accepts `rootId` only.                                                               |
| `readChildren`           | renderer-callable | Read-only hierarchy request.                                                                      |
| host status read/listen  | renderer-callable | Host availability surface.                                                                        |

No renderer-callable operation may accept `absolutePath` for root registration. `registerLocalRoot` must not be exposed
as renderer IPC, preload API, or renderer API.

## Future Generation

If exposure metadata becomes generated, extend the existing `library-boundary-protocol` and `xtask` export pipeline. Do
not add a parallel hand-written TypeScript operation registry or duplicate source of truth.

This decision does not introduce Exclave integration, new runtime machinery, or changes to the current IPC path.
