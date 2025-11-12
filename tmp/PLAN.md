# Migration Plan: Dhall to Python for Docker Configuration Generation

## Executive Summary

Migrate from Dhall to Python-based Docker Compose configuration generation to improve developer onboarding and maintainability while preserving type safety through Pydantic.

## Current State Analysis

### Dhall Configuration Structure
- **Total lines**: ~928 lines of Dhall code
- **Location**: `./docker/.config/`
- **Services**: 11 service definitions
  - neoprism
  - db (PostgreSQL)
  - cardano-node
  - cardano-wallet
  - cardano-dbsync
  - cardano-submit-api
  - prism-node
  - scala-did
  - ryo (Blockfrost backend)
  - caddy (reverse proxy)
  - uni-resolver-web
- **Stacks**: 3 stack compositions
  - prism-test
  - universal-resolver
  - blockfrost-neoprism-demo
- **Output**: 7 Docker Compose YAML files
  - mainnet-dbsync/compose.yml
  - mainnet-relay/compose.yml
  - preprod-relay/compose.yml
  - prism-test/compose.yml
  - prism-test/compose-ci.yml
  - blockfrost-neoprism-demo/compose.yml
  - mainnet-universal-resolver/compose.yml

### Current Generation Flow
```
./docker/.config/main.dhall → dhall-to-yaml → ./docker/*/compose.yml
```

Invoked via: `just build-config`

## Proposed Python Architecture

### Directory Structure
```
docker/
├── .config/
│   ├── main.py                    # Entry point for generation
│   ├── models.py                  # Pydantic models for type safety
│   ├── services/
│   │   ├── __init__.py
│   │   ├── neoprism.py
│   │   ├── db.py
│   │   ├── caddy.py
│   │   ├── cardano_node.py
│   │   ├── cardano_wallet.py
│   │   ├── cardano_dbsync.py
│   │   ├── cardano_submit_api.py
│   │   ├── prism_node.py
│   │   ├── scala_did.py
│   │   ├── ryo.py
│   │   └── uni_resolver_web.py
│   └── stacks/
│       ├── __init__.py
│       ├── prism_test.py
│       ├── universal_resolver.py
│       └── blockfrost_neoprism_demo.py
├── mainnet-dbsync/
│   └── compose.yml               # Generated file
├── mainnet-relay/
│   └── compose.yml               # Generated file
└── ... (other stacks)
```

### Core Components

#### 1. Pydantic Models (`models.py`)

Define type-safe models matching Docker Compose schema:

```python
from pydantic import BaseModel, Field
from typing import Dict, List, Literal

class Healthcheck(BaseModel):
    test: List[str]
    interval: str = "2s"
    timeout: str = "5s"
    retries: int = 30

class ServiceDependency(BaseModel):
    condition: Literal["service_started", "service_healthy", "service_completed_successfully"]

class Service(BaseModel):
    image: str
    restart: str | None = "always"
    ports: List[str] | None = None
    command: List[str] | None = None
    entrypoint: List[str] | None = None
    environment: Dict[str, str] | None = None
    volumes: List[str] | None = None
    depends_on: Dict[str, ServiceDependency] | None = None
    healthcheck: Healthcheck | None = None

class ComposeConfig(BaseModel):
    services: Dict[str, Service]
    volumes: Dict[str, Dict] | None = None
```

#### 2. Service Builders (e.g., `services/neoprism.py`)

Each service file mirrors the Dhall structure with typed Options:

```python
from pydantic import BaseModel
from typing import Literal
from ..models import Service, ServiceDependency, Healthcheck

class DbSyncDltSourceArgs(BaseModel):
    url: str
    poll_interval: int = 10

class RelayDltSource(BaseModel):
    type: Literal["relay"]
    address: str

class DbSyncDltSource(BaseModel):
    type: Literal["dbsync"]
    args: DbSyncDltSourceArgs

DltSource = RelayDltSource | DbSyncDltSource

class DltSink(BaseModel):
    wallet_host: str
    wallet_port: int
    wallet_id: str
    wallet_passphrase: str
    wallet_payment_address: str

class Options(BaseModel):
    image_override: str | None = None
    host_port: int | None = None
    db_host: str = "db"
    network: str = "mainnet"
    dlt_source: DltSource
    dlt_sink: DltSink | None = None
    confirmation_blocks: int | None = None
    index_interval: int | None = None
    extra_depends_on: list[str] = []

def mk_service(options: Options, version: str) -> Service:
    """Build neoprism service configuration."""
    # Implementation mirrors docker/.config/services/neoprism.dhall
    pass
```

#### 3. Stack Compositions (e.g., `stacks/prism_test.py`)

Complex multi-service stacks:

```python
from pydantic import BaseModel
from ..models import ComposeConfig
from .. import services

class Options(BaseModel):
    ci: bool = False

def mk_stack(options: Options, version: str) -> ComposeConfig:
    """Build prism-test stack configuration."""
    # Implementation mirrors docker/.config/stack/prism-test.dhall
    pass
```

#### 4. Main Generation Script (`main.py`)

```python
#!/usr/bin/env python3
"""Generate Docker Compose configurations from Python definitions."""

import yaml
from pathlib import Path
from typing import Any
from . import stacks
from . import services

def read_version() -> str:
    """Read version from root version file."""
    version_file = Path(__file__).parent.parent.parent / "version"
    return version_file.read_text().strip()

def write_compose_file(config: dict[str, Any], output_path: Path) -> None:
    """Write configuration to YAML file with header."""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w') as f:
        f.write("# Code generated by Python script. DO NOT EDIT.\n")
        yaml.dump(
            config,
            f,
            sort_keys=False,
            default_flow_style=False,
            allow_unicode=True
        )

def main() -> None:
    """Generate all Docker Compose configurations."""
    version = read_version()
    docker_dir = Path(__file__).parent.parent
    
    # Define all configurations
    configs = {
        "mainnet-dbsync": generate_mainnet_dbsync(version),
        "mainnet-relay": generate_mainnet_relay(version),
        "preprod-relay": generate_preprod_relay(version),
        "prism-test": stacks.prism_test.mk_stack(
            stacks.prism_test.Options(ci=False), 
            version
        ),
        "prism-test-ci": stacks.prism_test.mk_stack(
            stacks.prism_test.Options(ci=True), 
            version
        ),
        "blockfrost-neoprism-demo": stacks.blockfrost_neoprism_demo.mk_stack(
            stacks.blockfrost_neoprism_demo.Options(),
            version
        ),
        "mainnet-universal-resolver": stacks.universal_resolver.mk_stack(version),
    }
    
    # Generate all compose files
    for name, config in configs.items():
        # Handle both dict and Pydantic model
        if hasattr(config, 'model_dump'):
            config_dict = config.model_dump(exclude_none=True, by_alias=True)
        else:
            config_dict = config
            
        output_path = docker_dir / name / "compose.yml"
        write_compose_file(config_dict, output_path)
        print(f"Generated: {output_path}")

if __name__ == "__main__":
    main()
```

## Implementation Phases

### Phase 1: Setup Python Environment in Nix

**Tasks:**
1. Update `nix/devShells/development.nix`:
   - Add `python313` to packages
   - Add `python313Packages.pydantic` for type validation
   - Add `python313Packages.pyyaml` for YAML generation  
   - Add `pyright` for LSP support
2. Verify no `pyproject.toml` is created (as per requirements)
3. Test devshell loads correctly with `nix develop`

**Files Modified:**
- `nix/devShells/development.nix`

**Estimated Time:** 1 hour

### Phase 2: Create Python Configuration Structure

**Tasks:**
1. Create `docker/.config/models.py`:
   - Define Pydantic models for Docker Compose schema
   - Include: Service, Healthcheck, ServiceDependency, ComposeConfig
   - Ensure type safety matches Dhall's type system

2. Create service builders in `docker/.config/services/`:
   - Migrate each of 11 Dhall service files to Python
   - Preserve the same Options pattern for configurability
   - Keep the same function signatures (`mk_service`)
   - Services to migrate:
     - neoprism.py
     - db.py
     - caddy.py
     - cardano_node.py
     - cardano_wallet.py
     - cardano_dbsync.py
     - cardano_submit_api.py
     - prism_node.py
     - scala_did.py
     - ryo.py
     - uni_resolver_web.py

3. Create stack compositions in `docker/.config/stacks/`:
   - Migrate 3 stack definitions
   - Preserve complex composition logic
   - Stacks to migrate:
     - prism_test.py
     - universal_resolver.py
     - blockfrost_neoprism_demo.py

4. Create `docker/.config/main.py`:
   - Entry point for generation
   - Read version from `./version` file
   - Generate all 7 Docker Compose configurations
   - Write YAML with generated comment header

5. Create `docker/.config/__init__.py` (empty, marks as package)

**Files Created:**
- `docker/.config/models.py`
- `docker/.config/main.py`
- `docker/.config/__init__.py`
- `docker/.config/services/__init__.py`
- `docker/.config/services/*.py` (11 files)
- `docker/.config/stacks/__init__.py`
- `docker/.config/stacks/*.py` (3 files)

**Estimated Time:** 6-8 hours

### Phase 3: Integration with Just

**Tasks:**
1. Update `justfile`:
   - Modify `build-config` recipe to use Python instead of Dhall
   - New command: `python docker/.config/main.py`
   - Keep the same recipe name for backward compatibility
   
2. Update `format` recipe in `justfile`:
   - Remove Dhall formatting step (lines referencing `dhall format`)
   - Consider adding Python formatting if desired (e.g., `black` or `ruff`)
   - Optional: Add Python type checking step with `pyright`

3. Update `release-set` recipe:
   - Currently calls `just build-config` after version update
   - Should continue to work without changes

**Files Modified:**
- `justfile`

**Estimated Time:** 1 hour

### Phase 4: Testing & Validation

**Tasks:**
1. Generate configs and compare:
   - Run `just build-config` with new Python implementation
   - Compare generated YAML files against Dhall-generated ones
   - Use `diff` or YAML-aware comparison tools
   - Ensure semantic equivalence (field order may differ, which is OK)

2. Validate YAML syntax:
   - Ensure all 7 generated files are valid YAML
   - Check for proper indentation and structure

3. Test Docker stacks:
   - Run `just prism-test-up` and verify functionality
   - If possible, test other configurations
   - Ensure no regressions in behavior

4. Type checking:
   - Run `pyright` on Python configuration code
   - Fix any type errors

**Validation Checklist:**
- [ ] All 7 compose.yml files generated successfully
- [ ] Generated files are valid YAML
- [ ] Semantically equivalent to Dhall output
- [ ] prism-test stack starts successfully
- [ ] No type errors from Pyright
- [ ] All services have correct dependencies
- [ ] Environment variables preserved
- [ ] Health checks configured correctly

**Estimated Time:** 2-3 hours

### Phase 5: Cleanup & Documentation

**Tasks:**
1. Remove Dhall dependencies:
   - Remove `dhall` and `dhall-json` from `nix/devShells/development.nix`
   - Archive `docker/.config/*.dhall` files (move to `docker/.config/.dhall-archive/`)
   - Archive `docker/.config/services/*.dhall` files
   - Archive `docker/.config/stack/*.dhall` files

2. Update documentation:
   - Update AGENTS.md:
     - Remove Dhall formatting from code style section
     - Add Python configuration guidelines
   - Create `docker/.config/README.md`:
     - Document Python configuration structure
     - Explain how to add new services
     - Explain how to add new stacks
     - Document Options pattern

3. Optional: Add developer docs:
   - How to extend configurations
   - Type safety best practices
   - Common patterns

**Files Modified:**
- `nix/devShells/development.nix`
- `AGENTS.md`

**Files Created:**
- `docker/.config/README.md`
- `docker/.config/.dhall-archive/` (directory with archived files)

**Estimated Time:** 1 hour

## Benefits of Python Migration

### Developer Experience
✅ **Increased adoption**: Python is widely known by developers  
✅ **Easier onboarding**: Lower barrier to entry for contributions  
✅ **Better IDE support**: Excellent LSP support with Pyright  
✅ **Familiar syntax**: Standard Python dict/class syntax  

### Technical
✅ **Type safety**: Pydantic provides runtime validation similar to Dhall  
✅ **Maintainability**: Easier to debug and extend  
✅ **Flexibility**: Standard Python ecosystem and tooling  
✅ **No external language**: One less dependency to learn  

### Project Impact
✅ **Faster contributions**: Developers can modify configs without learning Dhall  
✅ **Better error messages**: Pydantic provides clear validation errors  
✅ **Standard tooling**: Use standard Python formatters, linters, type checkers  

## Risks & Mitigation

### Risk: Type safety at runtime vs compile-time
**Impact**: Medium  
**Mitigation**:
- Pydantic catches most issues at instantiation time
- Add validation tests for all configurations
- Use Pyright in strict mode for static type checking
- Run generation as part of CI to catch errors early

### Risk: Loss of Dhall's powerful type system
**Impact**: Low to Medium  
**Mitigation**:
- Use type hints extensively with Pyright in strict mode
- Leverage Pydantic's validation features (validators, field constraints)
- Use Union types and Literal types for enums
- Add custom validators where needed

### Risk: YAML generation differences
**Impact**: Low  
**Mitigation**:
- Carefully validate generated output matches semantically
- Use deterministic YAML generation settings (sort_keys=False)
- Test with actual Docker Compose to ensure functionality
- Document any intentional differences

### Risk: Python's dynamic nature
**Impact**: Low  
**Mitigation**:
- Mandatory type hints on all functions
- Use Pyright with strict mode
- Pydantic models enforce structure at runtime
- Add unit tests for service builders

## Estimated Effort

| Phase | Tasks | Estimated Time |
|-------|-------|----------------|
| Phase 1 | Nix setup | ~1 hour |
| Phase 2 | Python implementation | ~6-8 hours |
| Phase 3 | Just integration | ~1 hour |
| Phase 4 | Testing & validation | ~2-3 hours |
| Phase 5 | Cleanup & documentation | ~1 hour |
| **Total** | | **~11-14 hours** |

## Migration Strategy

### Incremental Approach

1. **Start with Phase 1** (Nix devshell updates)
2. **Create minimal working example**:
   - Implement `models.py`
   - Implement one simple service (e.g., `db.py`)
   - Implement one simple stack using that service
   - Generate one compose.yml and validate
3. **Iteratively add remaining services**:
   - Migrate services from simplest to most complex
   - Test each service as it's migrated
4. **Add remaining stacks**:
   - Complete stack implementations using migrated services
5. **Complete integration and testing**
6. **Final cleanup**

### Rollback Plan

If issues arise:
1. Keep Dhall files until Python implementation is validated
2. Archive Dhall files instead of deleting
3. Easy to revert `justfile` changes
4. Can temporarily support both implementations during transition

## Success Criteria

- [ ] All 7 Docker Compose files generate successfully
- [ ] Generated files are semantically identical to Dhall output
- [ ] `just build-config` works with Python implementation
- [ ] `just prism-test-up` starts stack successfully
- [ ] No Pyright type errors
- [ ] Documentation updated
- [ ] Dhall dependencies removed from devshell
- [ ] Team can understand and modify Python configs

## Next Steps

Once this plan is approved:

1. Begin Phase 1 implementation (Nix devshell setup)
2. Create minimal working example (models + one service)
3. Seek feedback on structure and approach
4. Continue with full implementation
5. Coordinate testing with team
6. Complete cleanup after validation

## References

### Files to Review During Implementation

- Current Dhall implementation: `docker/.config/**/*.dhall`
- Current justfile: `justfile` (lines 27-37)
- Current devshell: `nix/devShells/development.nix`
- Generated compose files: `docker/*/compose.yml`
- Version file: `version`

### Key Patterns to Preserve

1. **Options pattern**: Each service/stack has an Options class with defaults
2. **mk_service/mk_stack functions**: Build Service/ComposeConfig from Options
3. **Version reading**: Read from root `version` file
4. **Generated comment**: Add header to all generated YAML files
5. **Optional fields**: Many Docker Compose fields should be optional
6. **Environment variables**: Use dict/toMap pattern
7. **Dependencies**: Service dependency chains with health checks

### Python Best Practices

1. Use type hints everywhere
2. Use Pydantic for data validation
3. Use Union types for variants (like DltSource)
4. Use Literal types for string enums
5. Keep function signatures clean and documented
6. Add docstrings to public functions
7. Use pathlib for file operations
8. Make main.py executable with shebang
