"""Module docstring for sample."""

from dataclasses import dataclass
from typing import Optional


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


@dataclass
class Config:
    name: str
    count: int = 0
    enabled: bool = True


async def fetch_data(url: str, timeout: Optional[float] = None) -> bytes:
    """Fetch data from URL."""
    pass
