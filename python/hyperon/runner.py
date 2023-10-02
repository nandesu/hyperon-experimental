import os
from importlib import import_module
import importlib.util
import sys
import hyperonpy as hp
from .atoms import Atom, AtomType, OperationAtom
from .base import GroundingSpaceRef, Tokenizer, SExprParser

class RunnerState:
    def __init__(self, cstate):
        """Initialize a RunnerState"""
        self.cstate = cstate

    def __del__(self):
        """Frees a RunnerState and all associated resources."""
        hp.runner_state_free(self.cstate)

    def run_step(self):
        hp.metta_run_step(self.runner.cmetta, self.parser.cparser, self.cstate)

    def is_complete(self):
        return hp.runner_state_is_complete(self.cstate)

    def current_results(self, flat=False):
        """Returns the current in-progress results from an in-flight program evaluation"""
        results = hp.runner_state_current_results(self.cstate)
        if flat:
            return [Atom._from_catom(catom) for result in results for catom in result]
        else:
            return [[Atom._from_catom(catom) for catom in result] for result in results]

class MeTTa:
    """This class contains the MeTTa program execution utilities"""

    def __init__(self, space = None, cmetta = None):
        if cmetta is not None:
            self.cmetta = cmetta
        else:
            if space is None:
                space = GroundingSpaceRef()
            tokenizer = Tokenizer()
            self.cmetta = hp.metta_new(space.cspace, tokenizer.ctokenizer)
            self.load_py_module("hyperon.stdlib")
            hp.metta_load_module(self.cmetta, "stdlib")
            self.register_atom('extend-py!',
                OperationAtom('extend-py!',
                              lambda name: self.load_py_module_from_mod_or_file(name) or [],
                              [AtomType.UNDEFINED, AtomType.ATOM], unwrap=False))
            hp.metta_init_with_platform_env(self.cmetta)

    def __del__(self):
        hp.metta_free(self.cmetta)

    def space(self):
        """Gets the metta space"""
        return GroundingSpaceRef._from_cspace(hp.metta_space(self.cmetta))

    def tokenizer(self):
        """Gets the tokenizer"""
        return Tokenizer._from_ctokenizer(hp.metta_tokenizer(self.cmetta))

    def register_token(self, regexp, constr):
        """Registers a token"""
        self.tokenizer().register_token(regexp, constr)

    def register_atom(self, name, symbol):
        """Registers an Atom"""
        self.register_token(name, lambda _: symbol)

    def _parse_all(self, program):
        parser = SExprParser(program)
        while True:
            atom = parser.parse(self.tokenizer())
            if atom is None:
                break
            yield atom

    def parse_all(self, program):
        """Parse the entire program"""
        return list(self._parse_all(program))

    def parse_single(self, program):
        """Parse the next single line in the program"""
        return next(self._parse_all(program))

    def load_py_module(self, name):
        """Loads the given python module"""
        if not isinstance(name, str):
            name = repr(name)
        try:
            mod = import_module(name)
            for n in dir(mod):
                obj = getattr(mod, n)
                if '__name__' in dir(obj) and obj.__name__ == 'metta_register':
                    obj(self)
            return mod
        except:
            return None

    def load_py_module_from_mod_or_file(self, mod_name):
        """Loads the given python-implemented MeTTa module, first using python's module-namespace logic,
        then by searching for files in the MeTTa environment's search path"""

        # First, see if the module is already available to Python
        if not isinstance(mod_name, str):
            mod_name = repr(mod_name)
        mod = MeTTa.load_py_module(self, mod_name)
        if (mod is None):
            # If that failed, try and load the module from a file
            file_name = mod_name + ".py"

            # Check each search path directory in order, until we find the module we're looking for
            num_search_paths = hp.environment_search_path_cnt()
            search_path_idx = 0
            found_path = None
            while (search_path_idx < num_search_paths):
                search_path = hp.environment_nth_search_path(search_path_idx)
                test_path = os.path.join(search_path, file_name)
                if (os.path.exists(test_path)):
                    found_path = test_path
                    break
                search_path_idx += 1

            if (found_path is not None):
                MeTTa.load_py_module_from_path(self, mod_name, found_path)
            else:
                raise RuntimeError("Failed to load module " + mod_name + "; could not locate file: " + file_name)

    def load_py_module_from_path(self, mod_name, path):
        """Loads the given python-implemented MeTTa module from a file at the specified path"""

        spec = importlib.util.spec_from_file_location(mod_name, path)
        module = importlib.util.module_from_spec(spec)
        sys.modules[mod_name] = module
        spec.loader.exec_module(module)
        MeTTa.load_py_module(self, mod_name)

    def import_file(self, fname):
        """Loads the program file and runs it"""
        path = fname.split(os.sep)
        if len(path) == 1:
            path = ['.'] + path
        f = open(os.sep.join(path), "r")
        program = f.read()
        f.close()
        # changing cwd
        prev_cwd = os.getcwd()
        os.chdir(os.sep.join(path[:-1]))
        result = self.run(program)
        # restoring cwd
        os.chdir(prev_cwd)
        return result

    def run(self, program, flat=False):
        """Runs the program"""
        parser = SExprParser(program)
        results = hp.metta_run(self.cmetta, parser.cparser)
        if flat:
            return [Atom._from_catom(catom) for result in results for catom in result]
        else:
            return [[Atom._from_catom(catom) for catom in result] for result in results]

    def start_run(self, program):
        """Initializes a RunnerState to begin evaluation of MeTTa code"""
        parser = SExprParser(program)
        state = RunnerState(hp.metta_start_run(self.cmetta))
        state.parser = parser
        state.runner = self
        return state

class Environment:
    """This class contains the API for shared platform configuration"""

    def config_dir():
        """Returns the config dir in the platform environment"""
        return hp.environment_config_dir()
    def init_platform_env(working_dir = None, config_dir = None, disable_config = False, include_paths = []):
        """Initialize the platform environment with the supplied args"""
        hp.environment_init_start()
        if (working_dir is not None):
            hp.environment_init_set_working_dir(working_dir)
        if (config_dir is not None):
            hp.environment_init_set_config_dir(config_dir)
        if (disable_config):
            hp.environment_init_disable_config_dir()
        for path in reversed(include_paths):
            hp.environment_init_add_include_path(path)
        hp.environment_init_finish()

