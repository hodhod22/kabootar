# Kabootar SQL v2

Kabootar har en **inbyggd in-process databas** — ingen server, ingen connection string, samma lagring som `sql()` och `db` inom en session.

Designmål: PostgreSQL-kompatibel där det spelar roll, enklare att köra, och bättre integrerad med Kabootar-värden (JSON, SERIAL, UPSERT).

## Varför Kabootar DB?

| | PostgreSQL | Kabootar SQL v2 |
|---|------------|-----------------|
| Setup | Server + roller + extensions | `sql("…")` — klart |
| JSON | JSONB (bra) | Native `JSONB` → Kabootar-objekt |
| Auto-ID | `SERIAL` + sekvenser | `SERIAL PRIMARY KEY` inbyggt |
| Upsert | `ON CONFLICT` | ✅ Samma syntax |
| RETURNING | ✅ | ✅ INSERT, UPDATE, DELETE |
| Index scan | ✅ | ✅ PK + single/composite INDEX (EXPLAIN) |
| LEFT JOIN | ✅ | ✅ |
| GROUP BY | ✅ | ✅ SUM, AVG, MIN, MAX, COUNT |
| HAVING | ✅ | ✅ filter on aggregates |
| Transactions | ✅ | ✅ BEGIN / COMMIT / ROLLBACK / SAVEPOINT |
| Persistence | ✅ | ✅ WAL + SAVE/LOAD + `db_open()` |

## Anslutning

```kabootar
sql("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)")
```

## DDL

| SQL | Beskrivning |
|-----|-------------|
| `CREATE TABLE [IF NOT EXISTS] t (…)` | Skapa tabell med typer och constraints |
| `CREATE [UNIQUE] INDEX [IF NOT EXISTS] idx ON t (col)` | Index för snabbare lookup |
| `CREATE INDEX idx ON t (a, b)` | Composite index (flera kolumner) |
| `DROP TABLE t` | Ta bort tabell |

### Kolumntyper

`INTEGER`, `TEXT`, `FLOAT`, `BOOL`, `JSON`, `JSONB`, `SERIAL`

### Constraints

- `PRIMARY KEY` — unik radnyckel
- `NOT NULL` — obligatoriskt fält
- `UNIQUE` — unikt värde per kolumn
- `SERIAL` — auto-increment (INTEGER, NOT NULL)
- `REFERENCES other_table(col)` — foreign key
- `CHECK (col op value)` — t.ex. `CHECK (amount > 0)`

### ALTER TABLE

| SQL | Beskrivning |
|-----|-------------|
| `ALTER TABLE t ADD COLUMN col TYPE` | Lägg till kolumn |
| `ALTER TABLE t DROP COLUMN col` | Ta bort kolumn |
| `ALTER TABLE t RENAME COLUMN old TO new` | Byt kolumnnamn |

## DML

| SQL | Beskrivning |
|-----|-------------|
| `INSERT INTO t (cols) VALUES (vals)` | Infoga rad |
| `INSERT INTO t VALUES (...), (...)` | Batch-insert (flera rader) |
| `INSERT … ON CONFLICT DO NOTHING` | Ignorera dubblett (PK) |
| `INSERT … ON CONFLICT (col) DO UPDATE SET …` | Upsert på PK eller UNIQUE |
| `INSERT … RETURNING col, …` | Returnera infogad rad |
| `UPDATE t SET col = val WHERE …` | Uppdatera |
| `UPDATE … RETURNING col, …` | Returnera uppdaterade rader |
| `DELETE FROM t WHERE …` | Ta bort |
| `DELETE … RETURNING col, …` | Returnera borttagna rader |

## SELECT

| Feature | Exempel |
|---------|---------|
| WHERE | `WHERE id = $1 AND active = TRUE` |
| `IN` | `WHERE id IN (1, 2, 3)` |
| Subquery | `WHERE id IN (SELECT user_id FROM orders)` |
| `DISTINCT` | `SELECT DISTINCT dept FROM employees` |
| `LIKE` / `ILIKE` | `WHERE name LIKE 'Ada%'`, `WHERE name ILIKE 'ada'` |
| `BETWEEN` | `WHERE score BETWEEN 10 AND 20` |
| `NOT` | `WHERE NOT active = FALSE` |
| `IS NULL` | `WHERE note IS NULL` |
| JSON path | `WHERE body->>'title' = 'note'` |
| JSON contains | `WHERE body @> $1` |
| INNER / LEFT JOIN | `FROM a LEFT JOIN b ON a.id = b.id` |
| GROUP BY | `SELECT dept, SUM(amount) FROM s GROUP BY dept` |
| HAVING | `… GROUP BY dept HAVING SUM(amount) > 100` |
| Aggregat | `COUNT(*)`, `SUM`, `AVG`, `MIN`, `MAX` |
| ORDER BY | `ORDER BY score DESC` |
| LIMIT / OFFSET | `LIMIT 10 OFFSET 20` |

## Parametrar

```kabootar
sql("SELECT name FROM users WHERE id = $1", 1)
sql("INSERT INTO docs (body) VALUES ($1)", { "title": "note" })
```

## Transaktioner

```kabootar
sql("BEGIN")
sql("INSERT INTO accounts (id, balance) VALUES (1, 100)")
sql("SAVEPOINT before_transfer")
sql("UPDATE accounts SET balance = balance - 10 WHERE id = 1")
sql("ROLLBACK TO SAVEPOINT before_transfer")
sql("COMMIT")
```

| SQL | Beskrivning |
|-----|-------------|
| `BEGIN` / `BEGIN TRANSACTION` | Starta transaktion |
| `COMMIT` / `COMMIT TRANSACTION` | Spara ändringar |
| `ROLLBACK` / `ROLLBACK TRANSACTION` | Ångra allt sedan BEGIN |
| `SAVEPOINT name` | Checkpoint inom transaktion |
| `ROLLBACK TO SAVEPOINT name` | Ångra till checkpoint |
| `RELEASE SAVEPOINT name` | Ta bort checkpoint |

## Persistence

Spara och ladda hela databasen till disk:

```kabootar
sql("SAVE DATABASE 'myapp.kdb'")      -- JSON-format (v1)
sql("SAVE DATABASE 'myapp.kdb2'")     -- binärt KDB2-format (snabbare, kompakt)
sql("LOAD DATABASE 'myapp.kdb'")
sql("LOAD DATABASE 'myapp.kdb2'")     -- auto-detekterar KDB2 via magic "KDB2"
```

**KDB2** (`.kdb2`) använder kompakt radlagring (RowStore), B+tree-index och binär serialisering. WAL för KDB2 skrivs till `myapp.kdb2.wal2` (binär ramning) i stället för JSON-rader.

Öppna en persistent databas med WAL (skriver ändringar till `.wal` / `.wal2`, checkpoint vid COMMIT):

```kabootar
db_open("myapp.kdb")
sql("INSERT INTO users (name) VALUES ('Ada')")
sql("CHECKPOINT")   // flush .kdb och rensa WAL
sql("COMMIT")       // checkpoint automatiskt om db_open används
```

`db_open(path)` sätter globala `db`-anslutningen och återspelar WAL vid start. KDB2-filer detekteras automatiskt.

## Skalningsmotor (v4)

| Komponent | Beskrivning |
|-----------|-------------|
| **RowStore** | Kompakta rader per kolumnordning, tombstones vid DELETE |
| **B+tree** | Index med auto-index på PK/UNIQUE |
| **Buffer pool** | LRU-sidcache (8 KB-sidor) för KDB2 |
| **QueryPlanner** | Kostnadsbaserad EXPLAIN: Seq Scan, Index Scan, Index Only Scan |
| **ANALYZE** | `ANALYZE` / `ANALYZE tabell` — uppdaterar kolumnstatistik |
| **MVCC** | Snapshot-isolering inom BEGIN/COMMIT/ROLLBACK |
| **Prepared cache** | Parsade frågor cachas per SQL-sträng |
| **Parallell COUNT** | `COUNT(*)` på stora tabeller (>10k rader) |

```kabootar
sql("ANALYZE users")
sql("EXPLAIN SELECT email FROM users WHERE id = 1")
-- { plan: "Index Scan on users using PRIMARY (rows≈1)", rows: 1, cost: 1.0, index: "PRIMARY" }
sql("CHECKPOINT")   -- flush KDB2 + delta-sidor
```

## Diagnostik

```kabootar
sql("EXPLAIN SELECT name FROM users WHERE id = 1")
-- Returnerar objekt: { plan, rows, cost, index }
```

`plan` är t.ex. `"Index Scan on users (PRIMARY)"` eller `"Seq Scan on users"`.

## NULL-semantik

- `col = NULL` matchar **inte** — använd `IS NULL`
- `col IS NOT NULL` i WHERE

## Exempel

```kabootar
sql("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, email TEXT UNIQUE NOT NULL, meta JSONB)")
sql("INSERT INTO users (email, meta) VALUES ($1, $2) ON CONFLICT DO UPDATE SET meta = $2", "a@b.c", { "plan": "pro" })
sql("INSERT INTO users (email) VALUES ('x@y.z') RETURNING id")
sql("SELECT dept, SUM(amount) FROM sales GROUP BY dept")
sql("SELECT u.name, o.total FROM users u LEFT JOIN orders o ON u.id = o.user_id")
```

## Implementation

- Schema & constraints: `src/sql/schema.rs`
- JSON operators: `src/sql/json_ops.rs`
- WAL persistence: `src/sql/wal.rs`
- SQL-motor: `src/sql/mod.rs`
- Runtime: `src/runtime/db.rs`
