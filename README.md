# kubectl-why-can
Kubectl plugin to determine **why** a principal can perform an action. This is a simple wrapper around the Kubernetes `SelfSubjectAccessReview` API. The CLI format is modelled after the built-in `kubectl auth can-i`.

## Configuring log verbosity

Set environment variable `KUBECTL_WHY_CAN_LOG` to control the log verbosity.
By default, only errors are logged.

You can customize log verbosity for different subsystems.
For example, to enable debug logging only for the Kubernetes client library, you can use `KUBECTL_WHY_CAN_LOG="info,kube=debug"`.
