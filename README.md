# xycrd

Generate [Xymon][1] `hosts.cfg` snippets for services in [Kubernetes][2].

## Design Considerations

* A single configuration file allows inclusion via `option include <path>` directly in the main `hosts.cfg`, cleanup of the config file is not required.
* Multiple Kubernetes object updates are batched into a single rewrite of the configuration file with a configurable rewrite/reload delay. This incurs a certain delay for most updates, but this usually not really noticable.
* Cleanup (`drop ...`) is not implemented right now, as deletion of Endpoint is unusual. Version 2 might support it, though.
* This tool makes the simple case easy (a single URL check with optional content check), and provides an escape hatch to set additional Xymon tags.

Please note that there is no way to set global tags for all monitors. If this is needed, use `.default.` hosts prior to `optional include`, and reset them afterwards. See [`man hosts.cfg`][3] for reference.

## Installation

On Debian-compatible systems, use the package from the latest release and install it with `dpkg`.

On other systems, use the binary from the latest release and run it using `xymonlaunch`.

## Getting Started

1. After installation, `xycrd` stores the generated file (by default) in `/var/lib/xycrd/`. Include the snippet into the main `hosts.cfg`:  
```bash
optional include /var/lib/xycrd/xycrd.cfg
```
2. Add the CRD to Kubernetes:  
```bash
xycrd --print-install-files | kubectl apply -f -
```
3. Create `EndpointMonitor`s:
```yaml
kubectl apply -f - <<EOF
kind: EndpointMonitor
apiVersion: xycrd.thriqon.github.io/v1alpha1
metadata:
  name: icanhazip
spec:
  url: https://icanhazip.com/
EOF
```

## License

MIT License

Copyright (c) 2021 Jonas Weber

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

[1]: https://www.xymon.com/
[2]: https://kubernetes.io/
[3]: https://xymon.sourceforge.io/xymon/help/manpages/man5/hosts.cfg.5.html
