"""Module docstring for sample."""

from dataclasses import dataclass
from enum import Enum, IntEnum
from typing import Optional, Protocol

from pydantic import BaseModel


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


class Validator(Protocol):
    """A protocol for validators."""

    def validate(self, value: str) -> bool:
        ...


class Color:
    RED = 1
    GREEN = 2
    BLUE = 3


MAX_RETRIES: int = 3

DEFAULT_TIMEOUT: float = 30.0

_internal_cache: dict = {}


def _private_helper() -> None:
    pass
