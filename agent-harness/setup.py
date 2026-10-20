from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

with open("requirements.txt", "r", encoding="utf-8") as fh:
    requirements = [line.strip() for line in fh if line.strip() and not line.startswith("#")]

setup(
    name="cli-anything-eep",
    version="0.1.0",
    author="Easy Edits Pro Team",
    author_email="team@easy-edits-pro.org",
    description="Agent-native CLI for Easy Edits Pro video editor",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/zvirox/easy-edits-pro",
    packages=find_packages(include=["cli_anything_eep", "cli_anything_eep.*"]),
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: Apache Software License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Multimedia :: Video",
        "Topic :: Software Development :: Libraries :: Python Modules",
    ],
    python_requires=">=3.10",
    install_requires=requirements,
    entry_points={
        "console_scripts": [
            "cli-anything-eep=cli_anything_eep.cli:main",
        ],
    },
    include_package_data=True,
    package_data={
        "cli_anything_eep": ["skills/*.md"],
    },
)