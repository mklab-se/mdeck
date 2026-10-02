---
title: "Artifact Flow Tests"
@theme: light
---

# Artifact flow: a shared registry

```@artifactflow
# producers: Producing Teams | Build and publish artifacts
# consumers: Consuming Teams | Retrieve and use artifacts
- producer Build Team: Produces application binaries and container images
- producer Platform Team: Produces reusable libraries and platform packages
- service Artifactory: Artifact repository / registry
  - Container images
  - Binary packages
  - Libraries
- consumer Integration Team: Pulls images and libraries for test environments
- consumer Product Team: Pulls approved artifacts for production
- Build Team -> Artifactory: Container image v1.2.3 (icon: package)
- Platform Team -> Artifactory: Library package v4.5.0 (icon: code)
- Artifactory -> Integration Team: Pull image (icon: package)
- Artifactory -> Integration Team: Pull package (icon: code)
- Artifactory -> Product Team: Pull image (icon: package)
- Artifactory -> Product Team: Pull package (icon: code)
```

---

# Artifact flow: names only

```@artifactflow
- producer Backend
- producer Frontend
- producer Data
- service Registry
- consumer Staging
- consumer Production
```

---

# Artifact flow: two services

```@artifactflow
- producer Mobile Team: Builds the iOS and Android apps
- producer Web Team: Builds the web client
- producer API Team: Builds the public API
- service Package Registry: npm and Maven (icon: package)
- service Container Registry: OCI images (icon: container)
- consumer QA: Runs the release checks
- consumer Production: Serves customers (icon: cloud)
- Mobile Team -> Package Registry: SDK
- Web Team -> Package Registry: UI kit
- API Team -> Container Registry: api image
- Package Registry -> QA: test builds
- Container Registry -> QA: test images
- Container Registry -> Production: release images
```

---

# Artifact flow: progressive reveal

```@artifactflow
# producers: Builders
# services: Shared Infrastructure | Stores and serves artifacts
# consumers: Users
- producer CI Pipeline: Builds every commit (icon: function)
- service Artifact Store: Versioned and signed (icon: database)
+ CI Pipeline -> Artifact Store: build v2.0 (icon: package)
+ consumer Staging: Deploys every build
* Artifact Store -> Staging: latest
+ consumer Production: Deploys approved builds
* Artifact Store -> Production: approved
```

---

# Artifact flow: straight from producers to consumers

```@artifactflow
- producer Docs Team: Writes the handbook
- consumer Support: Answers customers
- consumer Sales: Runs demos
- Docs Team -> Support: handbook PDF
- Docs Team -> Sales: demo script
```
