# API Endpoints Specification

This document summarizes the proposed API endpoints based on the data available in `tamada/assets` and `tamadalab/assets`.

## Discovery

### `GET /api`
Returns a JSON object listing all available endpoints and their descriptions in English.

### `GET /api/ja`
Returns a JSON object listing all available endpoints and their descriptions in Japanese.

## Personal

These endpoints provide personal information about Haruaki Tamada.

### `GET /api/profile`
Returns the profile information of Haruaki Tamada.
- **Data Source:** `tamada/assets/profile.pkl`
- **Response Fields:** `id`, `name`, `ename`, `affiliation`, `category`, `generation`, `snss`, `icon`, `fromYear`, `degrees`.

### `GET /api/activities`
Returns a list of conference and academic activities.
- **Data Source:** `tamada/assets/activities.pkl`

### `GET /api/degrees`
Returns a list of academic degrees obtained.
- **Data Source:** `tamada/assets/degrees.pkl`

### `GET /api/job-histories`
Returns a list of career history entries.
- **Data Source:** `tamada/assets/job-histories.pkl`

### `GET /api/job-histories/{as}`
Returns a list of career history entries corresponding to the given `{as}`.

### `GET /api/job-histories/current`
Returns the current main career entry.

### `GET /api/skills`
Returns a list of technical and language skills.
- **Data Source:** `tamada/assets/skills.pkl`

---

## Laboratory

These endpoints provide information related to the Tamada Laboratory.

### `GET /api/members/all`
Returns a list of all lab members (current students, alumni, and collaborators).
- **Data Source:** `tamadalab/assets/members.pkl`

### `GET /api/members/{category}`
Returns a list of members filtered by category (e.g., `teachers`, `actives`, `almus`, `collaborators`).

### `GET /api/members/ids/{id}`
Returns detailed information about a specific lab member.
- **Note:** Excludes the `student_id` field for privacy.

### `GET /api/grants`
Returns a list of research grants (Kakenhi, etc.).
- **Data Source:** `tamadalab/assets/grants.pkl`

### `GET /api/grants/actives`
Returns a list of active research grants.

### `GET /api/papers`
Returns a list of all academic publications (excluding theses).
- **Data Source:** `tamadalab/assets/papers.pkl`

### `GET /api/papers/recent`
Returns the latest publications (e.g., the most recent 50 entries) for quick access.

### `GET /api/theses`
Returns a list of all bachelor, master, and doctoral theses from the lab.
- **Data Source:** `tamadalab/assets/theses.pkl`

### `GET /api/theses/years/{year}`
Returns theses published in a specific year.

### `GET /api/papers/types/{type}`
### `GET /api/papers/years/{year}`
### `GET /api/papers/languages/japanese`
Returns a list of publications written in Japanese.

### `GET /api/papers/languages/english`
Returns a list of publications written in English.

### `GET /api/papers/ids/{id}`
Returns details of a specific paper.

### `GET /api/activities`
Returns a list of laboratory activities.
- **Data Source:** `tamadalab/data/activities.json`
