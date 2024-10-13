# Wrapper
import os
from .pydms import *

CLIENT_PATH = os.path.join(os.path.dirname(__file__), "client")

__doc__ = pydms.__doc__
if hasattr(pydms, "__all__"):
    __all__ = pydms.__all__


# We cannot extend ServerState directly since it's a native class
class Server:
    def __init__(self):
        self.base = ServerState()

    def start(self, addr, static_path=None, graphs_path=None, experiments_path=None):
        if static_path is None:
            static_path = CLIENT_PATH
        self.base.start(addr, static_path, graphs_path, experiments_path)

    def __getattr__(self, name):
        return getattr(self.base, name)
