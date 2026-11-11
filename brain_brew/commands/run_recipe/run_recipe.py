import sys
from dataclasses import dataclass
from brain_brew.interfaces.command import Command
from brain_brew.commands.run_recipe.top_level_builder import TopLevelBuilder
from brain_brew.configuration.yaml_verifier import YamlVerifier


@dataclass
class RunRecipe(Command):
    recipe_file_name: str
    verify_only: bool

    def execute(self):
        # Parse Build Config File
        YamlVerifier()
        try:
            recipe = TopLevelBuilder.parse_and_read(self.recipe_file_name, self.verify_only)
        except FileNotFoundError:
            sys.stderr.write(f"error: Recipe file not found: '{self.recipe_file_name}'\n")
            sys.exit(2)

        if not self.verify_only:
            recipe.execute()
