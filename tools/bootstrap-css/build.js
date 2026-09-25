// Compiles Bootstrap 4.6.2 without print styles into src/assets/.
//
// The print styles (@page a3, body min-width 992px, white table
// backgrounds) break WeasyPrint, which always renders as print media.

const sass = require("sass");
const fs = require("fs");
const path = require("path");

const scss = `
$enable-print-styles: false;
@import "bootstrap/scss/bootstrap";
`;

const result = sass.compileString(scss, {
  loadPaths: [path.join(__dirname, "node_modules")],
  style: "expanded",
  quietDeps: true,
});

const out = path.join(
  __dirname,
  "..",
  "..",
  "src",
  "assets",
  "bootstrap-v4.6.2.css",
);
fs.writeFileSync(out, result.css);

console.log(`written ${out} (${result.css.length} bytes)`);
