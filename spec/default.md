# Pseudo-REST API Specification

This document defines the pseudo-REST API endpoints for accessing the data related to Haruaki Tamada and Tamada Laboratory.

## Overview

This API is designed as a **Static REST API**, primarily hosted on GitHub Pages. Data is served as pre-generated JSON files. Since it is static, complex server-side filtering via query parameters is replaced by client-side filtering or specific directory structures.

## Discovery
- `GET /api`: List of all available endpoints and descriptions (English).
- `GET /api/ja`: List of all available endpoints and descriptions (Japanese).

## 1. Personal

Personal data of Haruaki Tamada.

### Endpoints
- `GET /api/profile`: Returns personal profile details.
- `GET /api/activities`: Returns academic activities (conferences, etc.).
- `GET /api/degrees`: Returns academic degrees.
- `GET /api/job-histories`: Returns career history.
- `GET /api/skills`: Returns technical and language skills.

---

## 2. Laboratory

Data related to the Tamada Laboratory members and research output.

### Entities
- **Member**: Current students, alumni, and collaborators.
- **Paper**: Research publications (Journals, Conferences).
- **Thesis**: Bachelor, Master, and Doctoral theses.
- **Grant**: Research funding (Kakenhi, etc.).

### Endpoints

#### Members
- `GET /api/members/all`: Returns all members.
- `GET /api/members/{category}`: Filter members by category (e.g., `actives`, `almus`).
- `GET /api/members/ids/{id}`: Detailed info for a specific member.

#### Publications (Papers & Theses)
- `GET /api/papers`: All research publications.
- `GET /api/papers/recent`: Latest 50 publications for quick access.
- `GET /api/theses`: All graduation theses.
- `GET /api/theses/years/{year}`: Theses from a specific year.
- `GET /api/papers/years/{year}`: Publications from a specific year.
- `GET /api/papers/types/{type}`: Publications of a specific type.
- `GET /api/papers/languages/japanese`: Publications in Japanese.
- `GET /api/papers/languages/english`: Publications in English.

#### Grants & Activities
- `GET /api/grants`: All research grants.
- `GET /api/grants/actives`: List of active research grants.
- `GET /api/activities`: Lab-wide activities and events.

---

## Data Formats

All responses are in **JSON format**. 
Dates follow the `YYYY-MM-DD` or `YYYY-MM` standard.
Internal Pkl structures are mapped to JSON objects/arrays during the build process.

## API Usage Note
This API follows RESTful principles. Clients should access endpoints using the paths defined above (e.g., `/api/papers`). While the underlying data might be served statically, the interface remains a clean abstraction. For large resources, client-side filtering and sorting are recommended to minimize network overhead.
