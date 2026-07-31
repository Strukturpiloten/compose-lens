# Compose interpolation evidence

Reviewed: 2026-07-31.

## Documented behavior

The current Docker Compose reference and Compose Specification describe:

- unbraced `$VAR` and braced `${VAR}` direct substitution;
- `:-` and `-` default operators, which distinguish unset from empty values;
- `:?` and `?` required operators;
- `:+` and `+` alternative operators;
- nested interpolation operands;
- `$$` as an escaped dollar sign that prevents Compose interpolation;
- unsupported shell-style pattern substitution;
- a warning plus empty-string substitution for an unset direct variable;
- interpolation of YAML values rather than keys; and
- interpolation per file before merge.

Docker's environment-variable guide additionally documents that interpolation applies to
unquoted and double-quoted values, and documents Docker Compose CLI precedence between the shell,
explicit environment files, and the project `.env` file. It also states that `.env` substitution
is a Docker Compose CLI feature rather than a Swarm feature.

Sources:

- [Docker Compose interpolation reference](https://docs.docker.com/reference/compose-file/interpolation/)
- [Compose Specification interpolation chapter](https://github.com/compose-spec/compose-spec/blob/main/12-interpolation.md)
- [Docker Compose variable interpolation guide](https://docs.docker.com/compose/how-tos/environment-variables/variable-interpolation/)

## ComposeLens interpretation

ComposeLens implements the expression evaluator independently from environment discovery:

- the evaluator supports every documented operator and nested operand;
- a caller-supplied provider decides whether a variable is set and whether its value is sensitive;
- default direct-substitution behavior matches the documented warning-and-empty result;
- alternative recovery policies are explicit options;
- diagnostics use stable generic text and never include the required-expression message operand or
  a resolved value; and
- document application produces a non-destructive overlay for one supplied file; the loader remains
  responsible for file order, provider precedence, and merge provenance.

The authored `processing/interpolation-operators` fixture protects unset-versus-empty behavior,
nested defaults, alternatives, and escaped dollars. Integration tests separately protect required
errors, unsupported syntax, safety limits, custom providers, and sensitivity propagation.
