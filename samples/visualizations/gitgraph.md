---
title: "Git Graph Visualization"
@theme: dark
@transition: slide
---

# Git Graph — Basic

```@gitgraph
- lane main
- lane develop
- lane feature
- commit main
- branch main -> develop
- branch develop -> feature
- commit feature
- commit feature
- merge feature -> develop
- merge develop -> main
```

---

# Git Graph — Progressive Reveal

```@gitgraph
- lane main
- lane develop
- lane feature
- commit main
+ branch main -> develop
+ branch develop -> feature
+ commit feature
+ commit feature
+ merge feature -> develop: "PR #12"
+ merge develop -> main: "Release v2.0"
+ tag main: "v2.0"
```

---

# Git Flow

```@gitgraph
- lane main
- lane hotfix
- lane release
- lane develop
- lane feature
- commit main
- branch main -> develop
+ branch develop -> feature
+ commit feature
+ commit feature
+ merge feature -> develop
+ branch develop -> release
+ commit release
+ merge release -> main: "v1.0"
* merge release -> develop
+ tag main: "v1.0"
```

---

# Hotfix Flow

```@gitgraph
- lane main
- lane hotfix
- lane develop
- commit main
- branch main -> develop
- commit develop
+ branch main -> hotfix
+ commit hotfix
+ merge hotfix -> main: "v1.0.1"
* merge hotfix -> develop
+ tag main: "v1.0.1"
```

---

# Multiple Features

```@gitgraph
- lane main
- lane release
- lane develop
- lane feature/ui
- lane feature/api
- commit main
- branch main -> develop
+ branch develop -> feature/ui
* branch develop -> feature/api
+ commit feature/ui
+ commit feature/api
+ merge feature/ui -> develop: "PR #1"
+ merge feature/api -> develop: "PR #2"
+ branch develop -> release
+ merge release -> main: "v2.0"
* merge release -> develop
+ tag main: "v2.0"
```
