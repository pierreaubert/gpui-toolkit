"""V2 columnar resources and declarative data bindings.

Values deliberately never appear in ``to_spec``: a view binds the stable
resource identity and generation, while the session transport owns payload
publication.  The implementation is dependency-free; optional dataframe and
Arrow adapters are discovered only when callers use them.
"""
from __future__ import annotations

from collections.abc import Iterable, Mapping, Sequence
from contextlib import AbstractContextManager
from dataclasses import dataclass, field as dataclass_field
from hashlib import sha256
from typing import Any
from uuid import uuid4


class _Unset:
    def __repr__(self) -> str:
        return "UNSET"


UNSET = _Unset()


class DataError(ValueError):
    """Base class for path-qualified v2 data declaration errors."""


class SchemaError(DataError):
    pass


class ClosedResourceError(DataError):
    pass


class MutationError(DataError):
    pass
class DataTransportError(DataError):
    """A resource cannot be encoded for binary host publication."""
    pass


@dataclass(frozen=True)
class Expr:
    """A serializable expression AST; it never evaluates Python source."""

    op: str
    args: tuple[Any, ...]

    def _binary(self, op: str, other: Any) -> "Expr":
        return Expr(op, (self, _expr_value(other)))

    def __eq__(self, other: object) -> "Expr":  # type: ignore[override]
        return self._binary("eq", other)

    def __ne__(self, other: object) -> "Expr":  # type: ignore[override]
        return self._binary("ne", other)

    def __lt__(self, other: Any) -> "Expr":
        return self._binary("lt", other)

    def __le__(self, other: Any) -> "Expr":
        return self._binary("le", other)

    def __gt__(self, other: Any) -> "Expr":
        return self._binary("gt", other)

    def __ge__(self, other: Any) -> "Expr":
        return self._binary("ge", other)

    def __and__(self, other: Any) -> "Expr":
        return self._binary("and", other)

    def __or__(self, other: Any) -> "Expr":
        return self._binary("or", other)

    def __invert__(self) -> "Expr":
        return Expr("not", (self,))

    def is_null(self) -> "Expr":
        return Expr("is_null", (self,))

    def isin(self, values: Sequence[str | int | float | bool | None]) -> "Expr":
        if not values:
            raise DataError("isin requires at least one literal")
        return Expr("in", (self, tuple(_expr_value(value) for value in values)))

    def to_spec(self) -> dict[str, Any]:
        return {"op": self.op, "args": [_expr_spec(arg) for arg in self.args]}


def _expr_value(value: Any) -> Any:
    if isinstance(value, Expr) or value is None or isinstance(value, (str, int, float, bool)):
        return value
    raise DataError(f"expression literal has unsupported type {type(value).__name__}")


def _expr_spec(value: Any) -> Any:
    if isinstance(value, Expr):
        return value.to_spec()
    if isinstance(value, tuple):
        return [_expr_spec(item) for item in value]
    return value


def col(name: str) -> Expr:
    if not isinstance(name, str) or not name:
        raise DataError("column name must be a non-empty string")
    return Expr("field", (name,))


@dataclass(frozen=True)
class FieldRef:
    """Explicit table-template field reference, distinct from a filter AST."""

    name: str

    def __post_init__(self) -> None:
        if not self.name:
            raise DataError("field reference must be a non-empty string")

    def to_spec(self) -> dict[str, str]:
        return {"kind": "field_ref", "field": self.name}


def field(name: str) -> FieldRef:
    return FieldRef(name)


@dataclass(frozen=True)
class Categorical:
    """Dictionary-encoded column input without an optional Arrow dependency."""

    codes: Sequence[int | None]
    categories: Sequence[str]

    def values(self) -> tuple[str | None, ...]:
        if not self.categories:
            raise DataError("categorical column requires at least one category")
        values: list[str | None] = []
        for code in self.codes:
            if code is None:
                values.append(None)
            elif code < 0 or code >= len(self.categories):
                raise DataError("categorical code is outside category range")
            else:
                values.append(self.categories[code])
        return tuple(values)


@dataclass(frozen=True)
class DatasetView:
    dataset: "Dataset"
    operations: tuple[dict[str, Any], ...] = ()

    @property
    def available_fields(self) -> tuple[str, ...]:
        fields = self.dataset.schema
        group_fields: tuple[str, ...] = ()
        for operation in self.operations:
            kind = operation.get("op")
            if kind == "select":
                fields = tuple(operation["fields"])
            elif kind == "group_by":
                group_fields = tuple(operation["fields"])
            elif kind == "aggregate":
                fields = (*group_fields, *operation["aggregations"].keys())
                group_fields = ()
        return fields

    def _validate_fields(self, fields: Iterable[str]) -> None:
        available = set(self.available_fields)
        missing = [field for field in fields if field not in available]
        if missing:
            raise DataError(
                "DatasetView fields are unavailable: " + ", ".join(sorted(set(missing)))
            )

    def filter(self, expression: Expr) -> "DatasetView":
        if not isinstance(expression, Expr):
            raise DataError("DatasetView.filter requires a data.Expr")
        if any(operation.get("op") == "aggregate" for operation in self.operations):
            raise DataError("DatasetView.filter after aggregate is not supported")
        spec = expression.to_spec()
        def expression_fields(value: Any) -> list[str]:
            if not isinstance(value, dict):
                return []
            if value.get("op") == "field":
                args = value.get("args", ())
                return [args[0]] if args and isinstance(args[0], str) else []
            return [
                field
                for argument in value.get("args", ())
                for field in expression_fields(argument)
            ]

        self._validate_fields(expression_fields(spec))
        return self._with("filter", expression=spec)

    def select(self, *fields: str) -> "DatasetView":
        if not fields:
            raise DataError("select requires at least one field")
        if len(set(fields)) != len(fields):
            raise DataError("select fields must be unique")
        if self.selected_fields is not None:
            raise DataError("DatasetView supports one select operation")
        self._validate_fields(fields)
        return self._with("select", fields=list(fields))

    def sort(self, field: str, *, descending: bool = False) -> "DatasetView":
        self._validate_fields((field,))
        if any(operation.get("op") == "sort" for operation in self.operations):
            raise DataError("DatasetView supports one sort operation")
        if any(operation.get("op") == "range" for operation in self.operations):
            raise DataError("DatasetView.sort must precede range")
        if self.selected_fields is not None and field not in self.selected_fields:
            raise DataError("sort field is unavailable after DatasetView.select")
        return self._with("sort", field=field, descending=bool(descending))

    @property
    def selected_fields(self) -> tuple[str, ...] | None:
        for operation in self.operations:
            if operation.get("op") == "select":
                return tuple(operation["fields"])
        return None

    def group_by(self, *fields: str) -> "DatasetView":
        if not fields:
            raise DataError("group_by requires at least one field")
        if len(set(fields)) != len(fields):
            raise DataError("group_by fields must be unique")
        if any(operation.get("op") in {"group_by", "aggregate"} for operation in self.operations):
            raise DataError("DatasetView supports one grouping stage")
        self._validate_fields(fields)
        return self._with("group_by", fields=list(fields))

    def aggregate(self, **aggregations: str) -> "DatasetView":
        if not aggregations:
            raise DataError("aggregate requires at least one named aggregation")
        if any(operation.get("op") == "aggregate" for operation in self.operations):
            raise DataError("DatasetView supports one aggregation stage")
        supported = {"count", "sum", "mean", "min", "max", "first", "last"}
        encoded: dict[str, str] = {}
        group_fields = next(
            (
                tuple(operation["fields"])
                for operation in reversed(self.operations)
                if operation.get("op") == "group_by"
            ),
            (),
        )
        for output, expression in aggregations.items():
            if not output or ":" not in expression:
                raise DataError("aggregation must be output='operation:field'")
            if output in group_fields:
                raise DataError(f"aggregation output {output!r} conflicts with group field")
            operation, field = expression.split(":", 1)
            if operation not in supported:
                raise DataError(f"unsupported aggregation {operation!r}")
            if operation == "count" and field == "*":
                pass
            else:
                self._validate_fields((field,))
            encoded[output] = expression
        return self._with("aggregate", aggregations=encoded)

    def bin(self, field: str, *, count: int) -> "DatasetView":
        self._validate_fields((field,))
        if count <= 0:
            raise DataError("bin count must be positive")
        return self._with("bin", field=field, count=int(count))

    def window(self, *, size: int, step: int = 1) -> "DatasetView":
        if size <= 0 or step <= 0:
            raise DataError("window size and step must be positive")
        return self._with("window", size=int(size), step=int(step))

    def range(self, start: int, stop: int) -> "DatasetView":
        if start < 0 or stop < start:
            raise DataError("range must satisfy 0 <= start <= stop")
        return self._with("range", start=start, stop=stop)

    def _with(self, op: str, **payload: Any) -> "DatasetView":
        return DatasetView(self.dataset, self.operations + ({"op": op, **payload},))

    def to_spec(self) -> dict[str, Any]:
        return {"kind": "dataset_view", "dataset": self.dataset.to_spec(), "operations": list(self.operations)}


@dataclass(frozen=True)
class DataBinding:
    """Maps dataset fields to chart/table semantic roles."""

    source: "Dataset | DatasetView | ArrayData"
    roles: Mapping[str, str] = dataclass_field(default_factory=dict)

    def role(self, name: str, field_name: str) -> "DataBinding":
        if not name or not field_name:
            raise DataError("binding role and field must be non-empty strings")
        if isinstance(self.source, DatasetView):
            self.source._validate_fields((field_name,))
        elif isinstance(self.source, Dataset):
            self.source._validate_fields((field_name,))
        return DataBinding(self.source, {**self.roles, name: field_name})

    def x(self, field_name: str) -> "DataBinding":
        return self.role("x", field_name)

    def y(self, field_name: str) -> "DataBinding":
        return self.role("y", field_name)

    def y0(self, field_name: str) -> "DataBinding":
        return self.role("y0", field_name)

    def y2(self, field_name: str) -> "DataBinding":
        return self.role("y2", field_name)

    def color(self, field_name: str) -> "DataBinding":
        return self.role("color", field_name)

    def size(self, field_name: str) -> "DataBinding":
        return self.role("size", field_name)

    def label(self, field_name: str) -> "DataBinding":
        return self.role("label", field_name)

    def series(self, field_name: str) -> "DataBinding":
        return self.role("series", field_name)

    def dash(self, field_name: str) -> "DataBinding":
        return self.role("dash", field_name)

    def row_id(self, field_name: str) -> "DataBinding":
        return self.role("row_id", field_name)

    def tooltip(self, field_name: str) -> "DataBinding":
        return self.role("tooltip", field_name)

    def accessibility_description(self, field_name: str) -> "DataBinding":
        return self.role("accessibility_description", field_name)

    def to_spec(self) -> dict[str, Any]:
        return {"kind": "data_binding", "source": self.source.to_spec(), "roles": dict(self.roles)}


class Dataset:
    """Mutable, revisioned columnar data with immutable declarations around it."""

    def __init__(self, columns: Mapping[str, Sequence[Any]], *, key: str | None = None, id: str | None = None) -> None:
        if not columns:
            raise SchemaError("dataset requires at least one column")
        self.id = id or f"dataset-{uuid4().hex}"
        if not self.id.strip():
            raise DataError("dataset id must be non-empty")
        self._categorical_fields = {name for name, values in columns.items() if isinstance(values, Categorical)}
        self._columns = self._normalize_columns(columns)
        self.key = key
        if key is not None:
            self._validate_fields((key,))
            self._assert_unique_keys(self._columns[key])
        self.generation = 1
        self._closed = False
        self._listeners: list[Any] = []

    @classmethod
    def from_mapping(cls, columns: Mapping[str, Sequence[Any]], *, key: str | None = None, id: str | None = None) -> "Dataset":
        return cls(columns, key=key, id=id)

    @classmethod
    def from_rows(cls, rows: Iterable[Mapping[str, Any]], *, key: str | None = None, id: str | None = None) -> "Dataset":
        materialized = list(rows)
        if not materialized:
            raise SchemaError("dataset rows must not be empty; use from_mapping for an empty schema")
        fields = tuple(materialized[0])
        if any(tuple(row) != fields for row in materialized):
            raise SchemaError("all rows must contain the same ordered fields")
        return cls({field: [row[field] for row in materialized] for field in fields}, key=key, id=id)

    @classmethod
    def from_pandas(cls, frame: Any, *, key: str | None = None, id: str | None = None) -> "Dataset":
        if not hasattr(frame, "to_dict"):
            raise DataError("from_pandas requires a dataframe-like object with to_dict")
        return cls(frame.to_dict(orient="list"), key=key, id=id)

    @classmethod
    def from_polars(cls, frame: Any, *, key: str | None = None, id: str | None = None) -> "Dataset":
        if not hasattr(frame, "to_dict"):
            raise DataError("from_polars requires a dataframe-like object with to_dict")
        try:
            columns = frame.to_dict(as_series=False)
        except TypeError:
            columns = frame.to_dict()
        return cls(columns, key=key, id=id)

    @classmethod
    def from_arrow(cls, table: Any, *, key: str | None = None, id: str | None = None) -> "Dataset":
        if not hasattr(table, "to_pydict"):
            raise DataError("from_arrow requires a PyArrow-compatible table with to_pydict")
        return cls(table.to_pydict(), key=key, id=id)

    @classmethod
    def from_dataframe(cls, frame: Any, *, key: str | None = None, id: str | None = None) -> "Dataset":
        interchange = getattr(frame, "__dataframe__", None)
        if interchange is None:
            raise DataError("object does not implement the dataframe interchange protocol")
        exported = interchange()
        # When present, PyArrow's optional adapter consumes the standardized
        # interchange buffers without turning it into a wheel dependency.
        try:
            from pyarrow.interchange import from_dataframe as arrow_from_dataframe
        except ImportError:
            arrow_from_dataframe = None
        if arrow_from_dataframe is not None:
            return cls.from_arrow(arrow_from_dataframe(frame), key=key, id=id)
        to_pydict = getattr(exported, "to_pydict", None)
        if callable(to_pydict):
            return cls(to_pydict(), key=key, id=id)
        names = list(exported.column_names())
        # The interchange protocol has no universal Python-value extractor;
        # adapters can provide to_dict without making pandas a dependency.
        if not hasattr(frame, "to_dict"):
            raise DataError("dataframe interchange import requires a to_dict adapter in this build")
        return cls(frame.to_dict(orient="list"), key=key, id=id)

    @property
    def schema(self) -> tuple[str, ...]:
        return tuple(self._columns)

    @property
    def row_count(self) -> int:
        return len(next(iter(self._columns.values())))

    @property
    def schema_fingerprint(self) -> str:
        signature = "\x1f".join(f"{name}:{self.column_types[name]}" for name in self.schema)
        return sha256(signature.encode()).hexdigest()

    @property
    def column_types(self) -> dict[str, str]:
        return {name: "dictionary" if name in self._categorical_fields else self._infer_type(values) for name, values in self._columns.items()}

    def view(self) -> DatasetView:
        self._ensure_open()
        return DatasetView(self)

    def binding(self) -> DataBinding:
        self._ensure_open()
        return DataBinding(self)

    def append(self, batch: Mapping[str, Sequence[Any]] | Iterable[Mapping[str, Any]]) -> None:
        incoming = self._batch_columns(batch)
        merged = {name: self._columns[name] + incoming[name] for name in self.schema}
        if self.key is not None:
            self._assert_unique_keys(merged[self.key])
        self._commit(merged)

    def upsert(self, batch: Mapping[str, Sequence[Any]] | Iterable[Mapping[str, Any]], *, key: str | None = None) -> None:
        key = key or self.key
        if key is None:
            raise MutationError("upsert requires a declared primary key")
        self._validate_fields((key,))
        incoming = self._batch_columns(batch)
        self._assert_unique_keys(incoming[key])
        positions = {value: index for index, value in enumerate(self._columns[key])}
        rows = [{name: self._columns[name][index] for name in self.schema} for index in range(self.row_count)]
        for index, value in enumerate(incoming[key]):
            row = {name: incoming[name][index] for name in self.schema}
            if value in positions:
                rows[positions[value]] = row
            else:
                positions[value] = len(rows)
                rows.append(row)
        self._commit({name: tuple(row[name] for row in rows) for name in self.schema})

    def delete(self, keys: Iterable[Any]) -> None:
        if self.key is None:
            raise MutationError("delete requires a declared primary key")
        removed = set(keys)
        self._commit({name: tuple(value for index, value in enumerate(values) if self._columns[self.key][index] not in removed) for name, values in self._columns.items()})

    def replace(self, snapshot: Mapping[str, Sequence[Any]] | Iterable[Mapping[str, Any]]) -> None:
        columns = self._batch_columns(snapshot)
        if self.key is not None:
            self._assert_unique_keys(columns[self.key])
        self._commit(columns)

    def migrate(self, snapshot: Mapping[str, Sequence[Any]], *, key: str | None = None) -> None:
        """Explicitly replace a dataset with a changed schema and new revision."""
        self._ensure_open()
        columns = self._normalize_columns(snapshot)
        next_key = self.key if key is None else key
        if next_key is not None:
            if next_key not in columns:
                raise SchemaError(f"migration primary key {next_key!r} is not in the new schema")
            self._assert_unique_keys(columns[next_key])
        self._columns = columns
        self._categorical_fields = {name for name, values in snapshot.items() if isinstance(values, Categorical)}
        self.key = next_key
        self.generation += 1
        for listener in tuple(self._listeners):
            listener(self)

    def transaction(self) -> "DatasetTransaction":
        self._ensure_open()
        return DatasetTransaction(self)

    def close(self) -> None:
        self._closed = True
        self._columns = {}
        self._listeners.clear()

    def subscribe(self, callback: Any) -> Any:
        """Observe committed generations; returns an idempotent unsubscribe."""
        self._ensure_open()
        if not callable(callback):
            raise TypeError("dataset subscriber must be callable")
        self._listeners.append(callback)
        def unsubscribe() -> None:
            if callback in self._listeners:
                self._listeners.remove(callback)
        return unsubscribe

    def __enter__(self) -> "Dataset":
        self._ensure_open()
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        self.close()

    def to_spec(self) -> dict[str, Any]:
        self._ensure_open()
        return {"kind": "dataset", "id": self.id, "generation": self.generation, "schema": list(self.schema), "column_types": self.column_types, "schema_fingerprint": self.schema_fingerprint, "row_count": self.row_count, "key": self.key}

    def to_arrow_ipc(self) -> bytes:
        """Encode this generation as Arrow IPC without values in UI IR."""
        self._ensure_open()
        try:
            import pyarrow as pa  # type: ignore[import-not-found]
        except ImportError:
            try:
                from . import _native

                columns = [
                    (name, self.column_types[name], list(values))
                    for name, values in self._columns.items()
                ]
                return bytes(_native.dataset_arrow_ipc(columns))
            except (ImportError, AttributeError) as error:
                raise DataTransportError(
                    "Arrow IPC publication requires the gpui-toolkit native wheel "
                    "or optional dependency pyarrow"
                ) from error
            except (TypeError, ValueError) as error:
                raise DataTransportError(
                    f"dataset {self.id!r} built-in IPC encoding failed: {error}"
                ) from error
        try:
            table = pa.Table.from_pydict(
                {name: list(values) for name, values in self._columns.items()}
            )
            sink = pa.BufferOutputStream()
            with pa.ipc.new_stream(sink, table.schema) as writer:
                writer.write_table(table)
            return sink.getvalue().to_pybytes()
        except (TypeError, ValueError, pa.ArrowException) as error:
            raise DataTransportError(
                f"dataset {self.id!r} cannot encode Arrow IPC: {error}"
            ) from error

    def arrow_ipc_chunks(self, *, max_bytes: int = 16 * 1024 * 1024) -> tuple[bytes, ...]:
        """Return bounded IPC chunks in deterministic sequence order."""
        if not isinstance(max_bytes, int) or max_bytes <= 0 or max_bytes > 16 * 1024 * 1024:
            raise DataTransportError("max_bytes must be an integer in 1..16 MiB")
        payload = self.to_arrow_ipc()
        return tuple(payload[start : start + max_bytes] for start in range(0, len(payload), max_bytes)) or (b"",)

    def _commit(self, columns: Mapping[str, Sequence[Any]]) -> None:
        self._ensure_open()
        self._columns = self._normalize_columns(columns)
        self.generation += 1
        for listener in tuple(self._listeners):
            listener(self)

    def _batch_columns(self, batch: Mapping[str, Sequence[Any]] | Iterable[Mapping[str, Any]]) -> dict[str, tuple[Any, ...]]:
        self._ensure_open()
        if isinstance(batch, Mapping):
            columns = self._normalize_columns(batch)
        else:
            rows = list(batch)
            columns = self._normalize_columns({name: [row[name] for row in rows] for name in self.schema})
        if tuple(columns) != self.schema:
            raise SchemaError(f"dataset schema mismatch: expected {self.schema}, got {tuple(columns)}")
        return columns

    @staticmethod
    def _normalize_columns(columns: Mapping[str, Sequence[Any]]) -> dict[str, tuple[Any, ...]]:
        if not all(isinstance(name, str) and name for name in columns):
            raise SchemaError("column names must be non-empty strings")
        normalized = {name: (values.values() if isinstance(values, Categorical) else tuple(values)) for name, values in columns.items()}
        lengths = {len(values) for values in normalized.values()}
        if len(lengths) > 1:
            raise SchemaError("all dataset columns must have equal length")
        for name, values in normalized.items():
            Dataset._validate_column_values(name, values)
        return normalized

    @staticmethod
    def _validate_column_values(name: str, values: Sequence[Any]) -> None:
        non_null = [value for value in values if value is not None]
        if not non_null:
            return
        categories = {Dataset._value_category(value) for value in non_null}
        if categories <= {"int", "float"}:
            return
        if len(categories) != 1:
            raise SchemaError(f"column {name!r} has incompatible value categories: {', '.join(sorted(categories))}")

    @staticmethod
    def _value_category(value: Any) -> str:
        if isinstance(value, bool):
            return "bool"
        if isinstance(value, int):
            return "int"
        if isinstance(value, float):
            return "float"
        if isinstance(value, str):
            return "utf8"
        if isinstance(value, Mapping):
            return "struct"
        if isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
            return "list"
        if type(value).__module__ == "datetime":
            return "temporal"
        return type(value).__qualname__

    def _validate_fields(self, fields: Iterable[str]) -> None:
        unknown = [field for field in fields if field not in self._columns]
        if unknown:
            raise SchemaError(f"unknown dataset column(s): {', '.join(unknown)}")

    @staticmethod
    def _assert_unique_keys(values: Sequence[Any]) -> None:
        if len(set(values)) != len(values):
            raise SchemaError("primary key values must be unique")

    @staticmethod
    def _infer_type(values: Sequence[Any]) -> str:
        non_null = [item for item in values if item is not None]
        if not non_null:
            return "null"
        categories = {Dataset._value_category(item) for item in non_null}
        if categories <= {"int", "float"}:
            return "float64" if "float" in categories else "int64"
        value = non_null[0]
        if isinstance(value, bool):
            return "bool"
        if isinstance(value, str):
            return "utf8"
        if isinstance(value, Mapping):
            return "struct"
        if isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
            return "list"
        module = type(value).__module__
        if module == "datetime":
            return "temporal"
        return "opaque"

    def _ensure_open(self) -> None:
        if self._closed:
            raise ClosedResourceError(f"dataset {self.id!r} is closed")


class DatasetTransaction(AbstractContextManager["DatasetTransaction"]):
    def __init__(self, dataset: Dataset) -> None:
        self.dataset, self._columns, self._done = dataset, dict(dataset._columns), False

    def append(self, batch: Mapping[str, Sequence[Any]] | Iterable[Mapping[str, Any]]) -> "DatasetTransaction":
        temporary = Dataset(self._columns, key=self.dataset.key, id=self.dataset.id)
        temporary.append(batch)
        self._columns = temporary._columns
        return self

    def replace(self, snapshot: Mapping[str, Sequence[Any]] | Iterable[Mapping[str, Any]]) -> "DatasetTransaction":
        self._columns = self.dataset._batch_columns(snapshot)
        return self

    def upsert(self, batch: Mapping[str, Sequence[Any]] | Iterable[Mapping[str, Any]], *, key: str | None = None) -> "DatasetTransaction":
        temporary = Dataset(self._columns, key=self.dataset.key, id=self.dataset.id)
        temporary.upsert(batch, key=key)
        self._columns = temporary._columns
        return self

    def delete(self, keys: Iterable[Any]) -> "DatasetTransaction":
        temporary = Dataset(self._columns, key=self.dataset.key, id=self.dataset.id)
        temporary.delete(keys)
        self._columns = temporary._columns
        return self

    def commit(self) -> None:
        if self._done:
            raise MutationError("transaction is already closed")
        if self.dataset.key is not None:
            self.dataset._assert_unique_keys(self._columns[self.dataset.key])
        self.dataset._commit(self._columns)
        self._done = True

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        if exc_type is None and not self._done:
            self.commit()
        self._done = True


class ArrayData:
    """Revisioned dense-buffer descriptor; payload remains out of the UI IR."""

    def __init__(self, data: Any, *, shape: Sequence[int], dtype: str, id: str | None = None) -> None:
        self.id = id or f"array-{uuid4().hex}"
        self.shape = tuple(int(size) for size in shape)
        if not self.shape or any(size <= 0 for size in self.shape):
            raise DataError("array shape must contain positive dimensions")
        self.dtype = dtype
        self.generation = 1
        self._closed = False
        self._data = self._buffer(data)
        self._validate_buffer()
        self._listeners: list[Any] = []

    @classmethod
    def from_buffer(cls, data: Any, *, shape: Sequence[int], dtype: str, id: str | None = None) -> "ArrayData":
        return cls(data, shape=shape, dtype=dtype, id=id)

    @classmethod
    def from_numpy(cls, array: Any, *, id: str | None = None) -> "ArrayData":
        if not hasattr(array, "shape") or not hasattr(array, "dtype"):
            raise DataError("from_numpy requires a NumPy-compatible shape and dtype")
        return cls(array, shape=tuple(array.shape), dtype=str(array.dtype), id=id)

    @classmethod
    def from_dlpack(cls, source: Any, *, id: str | None = None) -> "ArrayData":
        """Import a DLPack producer through optional NumPy support."""
        try:
            import numpy
        except ImportError as error:
            raise DataError("from_dlpack requires optional NumPy support") from error
        try:
            array = numpy.from_dlpack(source)
        except (AttributeError, TypeError, ValueError) as error:
            raise DataError("from_dlpack requires a valid DLPack producer") from error
        return cls.from_numpy(array, id=id)

    def replace(self, data: Any, *, shape: Sequence[int] | None = None, dtype: str | None = None) -> None:
        self._ensure_open()
        if shape is not None:
            candidate = tuple(int(size) for size in shape)
            if candidate != self.shape:
                raise SchemaError("array shape changes require a new ArrayData resource")
        if dtype is not None and dtype != self.dtype:
            raise SchemaError("array dtype changes require a new ArrayData resource")
        self._data = self._buffer(data)
        self._validate_buffer()
        self.generation += 1
        for listener in tuple(self._listeners):
            listener(self)

    def close(self) -> None:
        self._closed = True
        self._data = memoryview(b"")
        self._listeners.clear()

    def subscribe(self, callback: Any) -> Any:
        self._ensure_open()
        if not callable(callback):
            raise TypeError("array subscriber must be callable")
        self._listeners.append(callback)
        def unsubscribe() -> None:
            if callback in self._listeners:
                self._listeners.remove(callback)
        return unsubscribe

    def __enter__(self) -> "ArrayData":
        self._ensure_open()
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        self.close()

    def to_spec(self) -> dict[str, Any]:
        self._ensure_open()
        return {
            "kind": "array_data",
            "id": self.id,
            "generation": self.generation,
            "shape": list(self.shape),
            "dtype": self.dtype,
            "byte_length": self._data.nbytes,
            "schema_fingerprint": self.schema_fingerprint,
        }

    @property
    def schema_fingerprint(self) -> str:
        signature = f"{self.dtype.lower()}:{','.join(map(str, self.shape))}"
        return sha256(signature.encode()).hexdigest()

    def binary_chunks(self, *, max_bytes: int = 16 * 1024 * 1024) -> tuple[bytes, ...]:
        """Return bounded raw-buffer chunks for the binary session channel."""
        self._ensure_open()
        if not isinstance(max_bytes, int) or not 0 < max_bytes <= 16 * 1024 * 1024:
            raise DataTransportError("max_bytes must be an integer in 1..16 MiB")
        if not self._data.c_contiguous:
            raise DataTransportError("ArrayData transport requires a C-contiguous buffer")
        try:
            payload = self._data.cast("B").tobytes()
        except (TypeError, ValueError) as error:
            raise DataTransportError("ArrayData transport requires a byte-addressable buffer") from error
        return tuple(payload[start : start + max_bytes] for start in range(0, len(payload), max_bytes)) or (b"",)

    @staticmethod
    def _buffer(data: Any) -> memoryview:
        try:
            return memoryview(data)
        except TypeError as error:
            raise DataError("ArrayData requires an object implementing the buffer protocol") from error

    def _validate_buffer(self) -> None:
        bytes_per_value = {
            "u8": 1, "uint8": 1, "i8": 1, "int8": 1, "bool": 1,
            "u16": 2, "uint16": 2, "i16": 2, "int16": 2, "f16": 2, "float16": 2,
            "u32": 4, "uint32": 4, "i32": 4, "int32": 4, "f32": 4, "float32": 4,
            "u64": 8, "uint64": 8, "i64": 8, "int64": 8, "f64": 8, "float64": 8,
        }.get(self.dtype.lower())
        if bytes_per_value is None:
            raise DataError(f"unsupported array dtype {self.dtype!r}")
        expected = bytes_per_value
        for dimension in self.shape:
            expected *= dimension
        if self._data.nbytes != expected:
            raise DataError(f"array buffer length {self._data.nbytes} does not match shape {self.shape} and dtype {self.dtype} ({expected} bytes expected)")

    def _ensure_open(self) -> None:
        if self._closed:
            raise ClosedResourceError(f"array {self.id!r} is closed")
