"""
Backend integration — calls real Easy Edits Pro Rust crates.

This module provides the bridge between the Python CLI and the Rust core.
Uses subprocess for CLI calls, or could use PyO3 for direct integration.
"""

import subprocess
import json
import shutil
from pathlib import Path
from typing import Dict, Any, List, Optional
from dataclasses import dataclass


@dataclass
class BackendInfo:
    """Information about available backends."""
    name: str
    version: str
    path: str
    capabilities: List[str]


def detect_backends() -> Dict[str, Any]:
    """Detect available Easy Edits Pro backends."""
    backends = {}
    
    # Check for saturn-cli (main CLI binary)
    saturn_cli = shutil.which('saturn-cli')
    if saturn_cli:
        try:
            result = subprocess.run([saturn_cli, '--version'], capture_output=True, text=True)
            backends['saturn-cli'] = {
                'path': saturn_cli,
                'version': result.stdout.strip(),
                'type': 'cli'
            }
        except Exception:
            pass
    
    # Check for saturn-mcp
    saturn_mcp = shutil.which('saturn-mcp')
    if saturn_mcp:
        backends['saturn-mcp'] = {
            'path': saturn_mcp,
            'type': 'mcp'
        }
    
    # Check for saturn-control
    saturn_control = shutil.which('saturn-control')
    if saturn_control:
        backends['saturn-control'] = {
            'path': saturn_control,
            'type': 'control'
        }
    
    # Check for cargo (for building from source)
    cargo = shutil.which('cargo')
    if cargo:
        backends['cargo'] = {'path': cargo, 'type': 'build'}
    
    return backends


class SaturnCLI:
    """Wrapper around saturn-cli binary."""
    
    def __init__(self, cli_path: Optional[str] = None):
        self.cli_path = cli_path or shutil.which('saturn-cli')
        if not self.cli_path:
            raise RuntimeError("saturn-cli not found. Install Easy Edits Pro or build from source.")
    
    def _run(self, args: List[str], input_data: Optional[Dict] = None) -> Dict[str, Any]:
        """Run saturn-cli command and return JSON output."""
        cmd = [self.cli_path, '--json'] + args
        
        stdin = None
        if input_data:
            stdin = json.dumps(input_data).encode()
        
        try:
            result = subprocess.run(
                cmd,
                input=stdin,
                capture_output=True,
                timeout=300,  # 5 min timeout
                check=False
            )
            
            if result.returncode != 0:
                error_msg = result.stderr.decode() if result.stderr else "Unknown error"
                raise RuntimeError(f"saturn-cli failed: {error_msg}")
            
            output = result.stdout.decode().strip()
            if output:
                return json.loads(output)
            return {}
            
        except json.JSONDecodeError as e:
            raise RuntimeError(f"Invalid JSON from saturn-cli: {e}")
        except subprocess.TimeoutExpired:
            raise RuntimeError("saturn-cli timed out")
    
    # Project commands
    def project_create(self, name: str, template: Optional[str] = None, path: Optional[str] = None) -> Dict:
        args = ['project', 'create', '--name', name]
        if template:
            args.extend(['--template', template])
        if path:
            args.extend(['--path', path])
        return self._run(args)
    
    def project_open(self, path: str) -> Dict:
        return self._run(['project', 'open', '--path', path])
    
    def project_save(self, path: Optional[str] = None) -> Dict:
        args = ['project', 'save']
        if path:
            args.extend(['--path', path])
        return self._run(args)
    
    def project_inspect(self, path: str) -> Dict:
        return self._run(['project', 'inspect', '--path', path])
    
    # Timeline commands
    def timeline_create(self, name: str, fps: float = 30, width: int = 1920, height: int = 1080) -> Dict:
        return self._run(['timeline', 'create', '--name', name, '--fps', str(fps), '--width', str(width), '--height', str(height)])
    
    def timeline_list(self) -> Dict:
        return self._run(['timeline', 'list'])
    
    def timeline_open(self, timeline_id: str) -> Dict:
        return self._run(['timeline', 'open', '--id', timeline_id])
    
    # Track commands
    def track_add(self, kind: str, name: Optional[str] = None, index: Optional[int] = None) -> Dict:
        args = ['track', 'add', '--kind', kind]
        if name:
            args.extend(['--name', name])
        if index is not None:
            args.extend(['--index', str(index)])
        return self._run(args)
    
    def track_remove(self, track_id: str) -> Dict:
        return self._run(['track', 'remove', '--id', track_id])
    
    def track_list(self) -> Dict:
        return self._run(['track', 'list'])
    
    # Clip commands
    def clip_add(self, source: str, track_id: str, timeline_start: int, timeline_duration: Optional[int] = None) -> Dict:
        args = ['clip', 'add', '--source', source, '--track', track_id, '--at', str(timeline_start)]
        if timeline_duration:
            args.extend(['--duration', str(timeline_duration)])
        return self._run(args)
    
    def clip_move(self, clip_id: str, track_id: str, timeline_start: int) -> Dict:
        return self._run(['clip', 'move', '--id', clip_id, '--track', track_id, '--at', str(timeline_start)])
    
    def clip_trim(self, clip_id: str, source_in: int, source_out: int) -> Dict:
        return self._run(['clip', 'trim', '--id', clip_id, '--in', str(source_in), '--out', str(source_out)])
    
    def clip_remove(self, clip_id: str) -> Dict:
        return self._run(['clip', 'remove', '--id', clip_id])
    
    # Render commands
    def render_queue(self, preset: str, output: str, timeline_id: Optional[str] = None) -> Dict:
        args = ['render', 'queue', '--preset', preset, '--output', output]
        if timeline_id:
            args.extend(['--timeline', timeline_id])
        return self._run(args)
    
    def render_status(self) -> Dict:
        return self._run(['render', 'status'])
    
    # Color commands
    def color_grade(self, clip_id: str, lift: List[float] = None, gamma: List[float] = None, gain: List[float] = None, **kwargs) -> Dict:
        args = ['color', 'grade', '--clip', clip_id]
        if lift:
            args.extend(['--lift'] + [str(x) for x in lift])
        if gamma:
            args.extend(['--gamma'] + [str(x) for x in gamma])
        if gain:
            args.extend(['--gain'] + [str(x) for x in gain])
        for k, v in kwargs.items():
            args.extend([f'--{k.replace("_", "-")}', str(v)])
        return self._run(args)
    
    # Audio commands
    def audio_mix(self, track_id: str, volume: Optional[float] = None, pan: Optional[float] = None) -> Dict:
        args = ['audio', 'mix', '--track', track_id]
        if volume is not None:
            args.extend(['--volume', str(volume)])
        if pan is not None:
            args.extend(['--pan', str(pan)])
        return self._run(args)
    
    # Media commands
    def media_import(self, path: str, bin_name: Optional[str] = None) -> Dict:
        args = ['media', 'import', '--path', path]
        if bin_name:
            args.extend(['--bin', bin_name])
        return self._run(args)
    
    def media_search(self, query: str) -> Dict:
        return self._run(['media', 'search', '--query', query])
    
    def media_list(self, bin_name: Optional[str] = None) -> Dict:
        args = ['media', 'list']
        if bin_name:
            args.extend(['--bin', bin_name])
        return self._run(args)
    
    # Template commands
    def template_create(self, name: str, type_: str, spec: str) -> Dict:
        return self._run(['template', 'create', '--name', name, '--type', type_, '--spec', spec])
    
    def template_list(self) -> Dict:
        return self._run(['template', 'list'])
    
    # Undo/Redo
    def undo(self) -> Dict:
        return self._run(['undo'])
    
    def redo(self) -> Dict:
        return self._run(['redo'])
    
    def history(self) -> Dict:
        return self._run(['history'])
    
    # Commands
    def commands_list(self) -> Dict:
        return self._run(['commands'])
    
    def command_execute(self, command_id: str, params: Dict) -> Dict:
        return self._run(['command', 'execute', '--id', command_id, '--params', json.dumps(params)])


# Singleton instance
_saturn_cli: Optional[SaturnCLI] = None


def get_saturn_cli() -> SaturnCLI:
    """Get or create SaturnCLI instance."""
    global _saturn_cli
    if _saturn_cli is None:
        _saturn_cli = SaturnCLI()
    return _saturn_cli