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

    def get_tag(self) -> tuple[str, str, str]:
        plat_name = os.environ.get("KLINT_PYTHON_PLAT_NAME")
        if not plat_name:
            _, _, plat_name = super().get_tag()
        return "py3", "none", plat_name


setup(cmdclass={"bdist_wheel": PlatformWheel})
