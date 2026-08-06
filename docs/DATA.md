# Data (`import "data"`)

Kab-first **DataFrame** module for analysis — pandas-class MVP on top of `science` natives (`df_*`, CSV, Apache Parquet + KPQT1, plots).

## Quick start

```kab
import "science"
import "data"

let df = from([
    ["north", "apple", 10.0],
    ["south", "pear", 4.0]
], ["region", "fruit", "qty"])

let g = groupby(df, "region", "qty", "sum")
let pv = pivot(df, "region", "fruit", "qty", "sum")
writeCsv("/tmp/sales.csv", df)
let again = readCsv("/tmp/sales.csv")
let fig = interactiveLine([1.0, 2.0, 1.5], "trend")
```

## API (MVP)

| Area | Functions |
|------|-----------|
| Frame | `from`, `fromRows`, `toRows`, `select`, `filter`, `groupby`, `join`, `pivot`, `aggregate`, `nrows`, `head`, `columns` |
| I/O | `readCsv` / `readCsvText`, `readJson` / `readJsonText`, `readParquet` (`.parquet` Apache / `.kpqt` KPQT1), `writeCsv`, `writeJson`, `writeParquet`, `describe` |
| Plot | `line`, `scatter`, `hist`, `spark`, `interactiveLine`, `interactiveScatter` |

`groupby` how: `mean` \| `sum` \| `count`. `pivot` / `aggregate` also support `min` \| `max` in Kab.

## Files

- `lib/data.kab` — `pub import` surface
- `lib/data/frame.kab`, `io.kab`, `plot.kab`
- `examples/data_analysis.kab`
- `tests/data_module.rs`

Lower layer: `science/df`, `science/io`, `science/data`. Roadmap: [ROADMAP.md](ROADMAP.md) **Våg DATA**.
