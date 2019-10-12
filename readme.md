# inliner

> Create a self contained html file that can be displayed correctly in a browser.

`inliner` is expected to be used with a build script that produces optimized
output, such as `yarn build`. Optimizing `css` and `js` and fetching remote
resources is outside the scope of this tool - at least for now.

## Features

- [x] Parse valid html text input into a tree structure.
- [x] Replace all links with the content of the corresponding file from disk.
- [x] Render new html with inlined content.

Text files are embedded directly.
Media files are embedded as base64 encoded data urls.
