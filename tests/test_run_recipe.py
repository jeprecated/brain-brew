import pytest

from brain_brew.commands.argument_reader import BBArgumentReader
from brain_brew.configuration.file_manager import FileManager
from brain_brew.configuration.yaml_verifier import YamlVerifier


@pytest.fixture()
def file_manager():
    FileManager.clear_instance()
    YamlVerifier.clear_instance()
    manager = FileManager()
    yield manager
    FileManager.clear_instance()
    YamlVerifier.clear_instance()


@pytest.mark.parametrize("arguments", [
    (["run", "no-such-recipe"]),
    (["run", "no-such-recipe", "--verify"]),
])
def test_run_missing_recipe_exits_cleanly(file_manager, capsys, arguments):
    command = BBArgumentReader(test_mode=True).get_parsed(arguments)
    with pytest.raises(SystemExit) as exc:
        command.execute()
    assert exc.value.code == 2
    assert "Recipe file not found" in capsys.readouterr().err
