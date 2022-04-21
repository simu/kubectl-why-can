# kubectl-why-can
Kubectl plugin to determine **why** a principal can perform an action. This is a simple wrapper around the Kubernetes `SelfSubjectAccessReview` API. The CLI format is modelled after the built-in `kubectl auth can-i`.
