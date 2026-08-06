import unittest
from gpui_toolkit.i18n import Language, TranslationCatalog
class I18nTests(unittest.TestCase):
 def test_language_and_fallback_contract(self):
  catalog=TranslationCatalog(Language.FRENCH,{"save":"Enregistrer"})
  self.assertEqual(catalog.get("save"),"Enregistrer")
  self.assertEqual(catalog.get("cancel",{"cancel":"Cancel"}),"Cancel")
  self.assertEqual(Language.JAPANESE.native_name,"Nihongo")
if __name__ == "__main__": unittest.main()
