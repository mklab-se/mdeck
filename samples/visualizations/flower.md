---
title: "Flower Tests"
theme: light
---

# Flower: a platform and its teams

```@flower
- center Development Platform: Shared capabilities and services (icon: database)
- petal Team 1: Builds product features and contributes back
- petal Team 2: Builds services and contributes back
- petal Team 3: Builds tools and services and contributes back
- petal Team 4: Builds components and contributes back
- petal Team 5: Builds features and contributes back
```

---

# Flower: names only

```@flower
- center Platform
- Payments
- Identity
- Data
- Mobile
```

---

# Flower: three petals

```@flower
- center Design System: Tokens, components and guidelines (icon: package)
- petal Web: Product pages and checkout (icon: browser)
- petal iOS: Native app (icon: mobile)
- petal Android: Native app (icon: mobile)
```

---

# Flower: links between petals

```@flower
- center Data Platform: Ingest, store and serve (icon: database)
- petal Payments: Card and invoice flows
- petal Identity: Accounts and sign-in
- petal Analytics: Dashboards and reports
- petal Fraud: Risk scoring
- Payments -> Fraud: scores
- Analytics -> Identity: reads
```

---

# Flower: progressive reveal

```@flower
- center Internal Developer Platform: Golden paths for every team (icon: cloud)
+ petal Checkout: Contributes the payment SDK
+ petal Search: Contributes the indexing pipeline
+ petal Mobile: Contributes the release train
+ petal Web: Contributes the design tokens
+ Checkout -> Search: uses
```

---

# Flower: many petals

```@flower
- center Shared Services
- Billing
- Catalog
- Checkout
- Identity
- Inventory
- Messaging
- Search
- Shipping
- Support
- Reviews
```
