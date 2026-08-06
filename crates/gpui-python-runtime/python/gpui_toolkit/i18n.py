"""Language and serializable translation catalog declarations."""
from __future__ import annotations
from dataclasses import dataclass, field
from enum import Enum

class Language(str, Enum):
    ENGLISH="en"; FRENCH="fr"; GERMAN="de"; SPANISH="es"; JAPANESE="ja"
    @property
    def native_name(self) -> str:
        return {self.ENGLISH:"English",self.FRENCH:"Francais",self.GERMAN:"Deutsch",self.SPANISH:"Espanol",self.JAPANESE:"Nihongo"}[self]

@dataclass(frozen=True)
class TranslationCatalog:
    language: Language
    messages: dict[str, str] = field(default_factory=dict)
    fallback: Language = Language.ENGLISH
    def __post_init__(self) -> None:
        if any(not key.strip() or not value for key,value in self.messages.items()): raise ValueError("translation keys and messages must be non-empty")
    def get(self, key: str, fallback_messages: dict[str, str] | None = None) -> str:
        if key in self.messages: return self.messages[key]
        if fallback_messages is not None and key in fallback_messages: return fallback_messages[key]
        return "???"
    def to_spec(self) -> dict[str, object]: return {"language":self.language.value,"messages":self.messages,"fallback":self.fallback.value}
