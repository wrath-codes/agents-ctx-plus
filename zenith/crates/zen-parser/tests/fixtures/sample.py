"""Module docstring for sample."""

from __future__ import annotations

import functools
from abc import ABCMeta, abstractmethod
from contextlib import contextmanager
from dataclasses import dataclass, field
from enum import Enum, IntEnum, StrEnum
from functools import cached_property
from typing import (
    Generic,
    NamedTuple,
    Optional,
    ParamSpec,
    Protocol,
    TypeAlias,
    TypedDict,
    TypeVar,
    overload,
)

from pydantic import BaseModel

# ---------------------------------------------------------------------------
# Module-level dunder constants
# ---------------------------------------------------------------------------
__all__ = ["BaseProcessor", "Config", "fetch_data", "Status", "MAX_RETRIES"]
__version__ = "2.0.0"

# ---------------------------------------------------------------------------
# Untyped module-level constants
# ---------------------------------------------------------------------------
VERSION = "1.0.0"
DEBUG = False

# ---------------------------------------------------------------------------
# Typed module-level constants
# ---------------------------------------------------------------------------
MAX_RETRIES: int = 3
DEFAULT_TIMEOUT: float = 30.0
_internal_cache: dict = {}

# ---------------------------------------------------------------------------
# Type aliases & type variables
# ---------------------------------------------------------------------------
JsonValue: TypeAlias = (
    dict[str, "JsonValue"] | list["JsonValue"] | str | int | float | bool | None
)
T = TypeVar("T")
P = ParamSpec("P")

# ---------------------------------------------------------------------------
# Lambda assignment
# ---------------------------------------------------------------------------
double = lambda x: x * 2


# ===========================================================================
# Original classes
# ===========================================================================
class BaseProcessor:
    """A base processor class."""

    def process(self, data: list[str]) -> dict[str, int]:
        """Process data and return counts.

        Args:
            data: List of strings to process.

        Returns:
            A dictionary mapping strings to counts.
        """
        return {}

    @staticmethod
    def helper() -> None:
        pass

    @property
    def name(self) -> str:
        return "base"

    @classmethod
    def create(cls, name: str) -> "BaseProcessor":
        return cls()


@dataclass
class Config:
    name: str
    count: int = 0
    enabled: bool = True


class UserSettings(BaseModel):
    """Pydantic model for user settings."""

    username: str
    theme: str = "dark"
    notifications: bool = True


class Status(Enum):
    """Status enumeration."""

    ACTIVE = "active"
    INACTIVE = "inactive"
    PENDING = "pending"


class Priority(IntEnum):
    LOW = 1
    MEDIUM = 2
    HIGH = 3


class ProcessingError(Exception):
    """Raised when processing fails."""

    def __init__(self, message: str, code: int = 0):
        super().__init__(message)
        self.code = code


class ValidationError(ValueError):
    """Raised when validation fails."""

    pass


class Validator(Protocol):
    """A protocol for validators."""

    def validate(self, value: str) -> bool:
        ...


class Color:
    RED = 1
    GREEN = 2
    BLUE = 3


# ===========================================================================
# Original functions
# ===========================================================================
async def fetch_data(url: str, timeout: Optional[float] = None) -> bytes:
    """Fetch data from URL."""
    pass


def transform(*args, **kwargs) -> None:
    """A function with variadic args.

    :param args: Positional arguments.
    :param kwargs: Keyword arguments.
    :returns: Nothing.
    :raises ValueError: If input is invalid.
    """
    pass


def generate_items(count: int):
    """Generate items lazily.

    Yields:
        Sequential integers from 0 to count.
    """
    for i in range(count):
        yield i


def _private_helper() -> None:
    pass


# ===========================================================================
# Abstract class with metaclass
# ===========================================================================
class AbstractHandler(metaclass=ABCMeta):
    """An abstract handler with metaclass."""

    @abstractmethod
    def handle(self, request: dict) -> bool:
        """Handle a request."""
        ...

    @abstractmethod
    async def handle_async(self, request: dict) -> bool:
        """Handle a request asynchronously."""
        ...


# ===========================================================================
# Multiple inheritance
# ===========================================================================
class MultiBase(BaseProcessor, AbstractHandler):
    """A class with multiple inheritance."""

    def process(self, data: list[str]) -> dict[str, int]:
        return {}

    def handle(self, request: dict) -> bool:
        return True

    async def handle_async(self, request: dict) -> bool:
        return True


# ===========================================================================
# NamedTuple
# ===========================================================================
class Point(NamedTuple):
    """A 2D point."""

    x: float
    y: float
    label: str = "origin"


# ===========================================================================
# TypedDict
# ===========================================================================
class UserProfile(TypedDict):
    """User profile data."""

    name: str
    age: int
    email: str


# ===========================================================================
# __slots__ class
# ===========================================================================
class SlottedClass:
    """A class with __slots__."""

    __slots__ = ("x", "y", "name")

    def __init__(self, x: int, y: int, name: str):
        self.x = x
        self.y = y
        self.name = name


# ===========================================================================
# Nested classes
# ===========================================================================
class Outer:
    """Outer class."""

    class Inner:
        """Inner nested class."""

        def inner_method(self) -> str:
            return "inner"

    def outer_method(self) -> str:
        return "outer"


# ===========================================================================
# Nested functions
# ===========================================================================
def outer_function(x: int) -> int:
    """A function with nested function."""

    def inner_function(y: int) -> int:
        return x + y

    return inner_function(x)


# ===========================================================================
# Properties with setter/deleter
# ===========================================================================
class PropertyExample:
    """Class demonstrating properties."""

    def __init__(self, value: int):
        self._value = value

    @property
    def value(self) -> int:
        """Get the value."""
        return self._value

    @value.setter
    def value(self, new_value: int) -> None:
        self._value = new_value

    @value.deleter
    def value(self) -> None:
        del self._value


# ===========================================================================
# Overloaded function
# ===========================================================================
@overload
def parse_input(value: str) -> str: ...


@overload
def parse_input(value: int) -> int: ...


def parse_input(value):
    """Parse input with overloads."""
    return value


# ===========================================================================
# Context manager
# ===========================================================================
@contextmanager
def managed_resource(name: str):
    """A context manager function.

    Args:
        name: Resource name.

    Yields:
        The managed resource handle.
    """
    yield name


# ===========================================================================
# Positional-only and keyword-only params
# ===========================================================================
def mixed_params(pos_only: int, /, normal: str, *, kw_only: bool = False) -> None:
    """Function with positional-only and keyword-only params."""
    pass


# ===========================================================================
# Container with dunder methods
# ===========================================================================
class Container:
    """A container class with dunder methods."""

    def __init__(self, items: list):
        self.items = items

    def __len__(self) -> int:
        return len(self.items)

    def __getitem__(self, index: int):
        return self.items[index]

    def __iter__(self):
        return iter(self.items)

    def __repr__(self) -> str:
        return f"Container({self.items!r})"

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        pass


# ===========================================================================
# Generic class
# ===========================================================================
class Stack(Generic[T]):
    """A generic stack implementation."""

    def __init__(self) -> None:
        self._items: list[T] = []

    def push(self, item: T) -> None:
        self._items.append(item)

    def pop(self) -> T:
        return self._items.pop()


# ===========================================================================
# Async generator
# ===========================================================================
async def async_generate(count: int):
    """An async generator.

    Yields:
        Sequential integers.
    """
    for i in range(count):
        yield i


# ===========================================================================
# Multiple decorators
# ===========================================================================
def log_calls(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


def validate_args(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


@log_calls
@validate_args
def multi_decorated() -> None:
    """Function with multiple decorators."""
    pass


# ===========================================================================
# NumPy-style docstring
# ===========================================================================
def numpy_documented(x: float, y: float) -> float:
    """Compute the hypotenuse.

    Parameters
    ----------
    x : float
        The x coordinate.
    y : float
        The y coordinate.

    Returns
    -------
    float
        The hypotenuse length.

    Raises
    ------
    ValueError
        If x or y is negative.

    Examples
    --------
    >>> numpy_documented(3.0, 4.0)
    5.0

    Notes
    -----
    Uses the Pythagorean theorem.
    """
    return (x**2 + y**2) ** 0.5


# ===========================================================================
# Cached property
# ===========================================================================
class CachedExample:
    """Class with cached_property."""

    @cached_property
    def expensive(self) -> list[int]:
        """An expensive computation."""
        return list(range(1000))


# ===========================================================================
# Enum with methods
# ===========================================================================
class Direction(Enum):
    """Direction enumeration with methods."""

    NORTH = "N"
    SOUTH = "S"
    EAST = "E"
    WEST = "W"

    def opposite(self) -> "Direction":
        """Return the opposite direction."""
        opposites = {
            self.NORTH: self.SOUTH,
            self.SOUTH: self.NORTH,
            self.EAST: self.WEST,
            self.WEST: self.EAST,
        }
        return opposites[self]


# ===========================================================================
# StrEnum
# ===========================================================================
class HttpMethod(StrEnum):
    GET = "GET"
    POST = "POST"
    PUT = "PUT"
    DELETE = "DELETE"


# ===========================================================================
# Frozen dataclass with complex fields
# ===========================================================================
@dataclass(frozen=True)
class ImmutableConfig:
    """An immutable configuration."""

    name: str
    tags: list[str] = field(default_factory=list)
    metadata: dict[str, str] = field(default_factory=dict)


# ===========================================================================
# Visibility conventions
# ===========================================================================
class VisibilityExample:
    """Class demonstrating Python visibility conventions."""

    def public_method(self) -> None:
        pass

    def _protected_method(self) -> None:
        pass

    def __private_method(self) -> None:
        pass

    def __dunder_method__(self) -> None:
        pass


# ===========================================================================
# Async context manager class
# ===========================================================================
class AsyncResource:
    """An async context manager."""

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        pass

    async def fetch(self, url: str) -> bytes:
        return b""
