import os

from setuptools import setup
from wheel.bdist_wheel import bdist_wheel


class PlatformWheel(bdist_wheel):
    def finalize_options(self) -> None:
        super().finalize_options()
        self.root_is_pure = False
        plat_name = os.environ.get("KLINT_PYTHON_PLAT_NAME")
        if plat_name:
            self.plat_name = plat_name


setup(cmdclass={"bdist_wheel": PlatformWheel})
