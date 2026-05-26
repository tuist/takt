// Starter Takt handler.
//
// Takt invokes this file with Node and the following environment:
//   TAKT_RUN_ID         the run identifier
//   TAKT_CAPABILITY     the capability name (e.g. "example.run")
//   TAKT_PACKAGE_ROOT   absolute path to the package root
//   TAKT_INPUT_PATH     path to a JSON file containing the merged inputs
//   TAKT_RESULT_PATH    path the handler MUST write its result to
//
// The result is JSON with optional `output` and `artifacts`:
//
//   {
//     "output": <any>,                      // optional immediate value
//     "artifacts": [                        // optional persisted records
//       {
//         "name": "summary",
//         "type": "resource",               // or "file"
//         "value": <any>,                   // required when type=resource
//         "path": "<filesystem path>",      // required when type=file
//         "content_type": "application/json", // optional
//         "tags": { "kind": "example" }     // optional
//       }
//     ]
//   }

import { readFile, writeFile } from "node:fs/promises";

const inputPath = process.env.TAKT_INPUT_PATH;
const resultPath = process.env.TAKT_RESULT_PATH;

if (!inputPath || !resultPath) {
  console.error(
    "this handler is meant to be run by Takt; missing TAKT_INPUT_PATH / TAKT_RESULT_PATH",
  );
  process.exit(2);
}

const inputs = JSON.parse(await readFile(inputPath, "utf8"));

const result = {
  output: {
    greeting: `hello from ${process.env.TAKT_CAPABILITY}`,
    received: inputs,
  },
  artifacts: [
    {
      name: "summary",
      type: "resource",
      value: {
        run_id: process.env.TAKT_RUN_ID,
        input_keys: Object.keys(inputs),
      },
      tags: { kind: "example" },
    },
  ],
};

await writeFile(resultPath, JSON.stringify(result));
