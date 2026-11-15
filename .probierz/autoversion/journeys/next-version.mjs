#!/usr/bin/env node
// next-version: a maintainer asks autoversion what the next version is from
// what was published and what is proposed. With the repository's own
// released-surface.json as the published surface (the source of truth):
// the same surface is `internal`; a surface with one more name is
// `additive`; a surface missing a name is `breaking` and names it; a
// declared breakage escalates an internal change and never lowers a
// breaking one; while the major slot is zero breaking moves the minor slot
// and the others the patch, and from 1.0.0 breaking moves the major and
// additive the minor (docs/SPEC.md); a version that is not three slots is
// refused.
//
// The run names TUI_CMD (the autoversion binary) and WISENT_WORKSPACE (the
// folder holding the AutoVersion checkout).
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

function required(name) {
  const value = process.env[name]?.trim();
  if (!value) {
    process.stderr.write(`${name} is not set: run this journey through \`probierz run tui --app autoversion\`\n`);
    process.exit(2 /* https://www.gnu.org/software/bash/manual/html_node/Exit-Status.html */);
  }
  return value;
}

const autoversion = required('TUI_CMD');
const released = JSON.parse(readFileSync(join(required('WISENT_WORKSPACE'), 'AutoVersion', 'released-surface.json'), 'utf8'));
const own = join(required('PROBIERZ_ARTIFACTS'), 'autoversion');
mkdirSync(own, { recursive: true });
const surface = (name, names) => {
  const path = join(own, `${name}.json`);
  writeFileSync(path, JSON.stringify({ surface: names }));
  return path;
};
const published = surface('published', released.surface);
const decide = (current, candidate, breaking = false) => {
  const result = spawnSync(autoversion, ['--json', 'decide', '--current', current, '--published-surface', published, '--candidate-surface', candidate, ...(breaking ? ['--breaking'] : [])], { encoding: 'utf8' });
  let json = {};
  try {
    json = JSON.parse(result.stdout);
  } catch {
    // a refusal prints on stderr
  }
  return { status: result.status, json, stderr: result.stderr };
};

const removedName = released.surface[released.surface.length - 1];
const same = surface('same', released.surface);
const added = surface('added', [...released.surface, 'cmd:autoversion explain']);
const removed = surface('removed', released.surface.slice(0, -1));

const broken = [];
const expect = (holds, sentence) => {
  if (!holds) broken.push(sentence);
};
const cases = [
  ['0.1.0', same, false, 'internal', '0.1.1'],
  ['0.1.0', added, false, 'additive', '0.1.1'],
  ['0.1.0', removed, false, 'breaking', '0.2.0'],
  ['0.1.0', same, true, 'breaking', '0.2.0'],
  ['1.4.2', added, false, 'additive', '1.5.0'],
  ['1.4.2', removed, false, 'breaking', '2.0.0'],
  ['1.4.2', removed, true, 'breaking', '2.0.0'],
];
for (const [current, candidate, breaking, change, next] of cases) {
  const answer = decide(current, candidate, breaking);
  expect(answer.status === 0 && answer.json.change === change && answer.json.next === next, `${current} with the ${candidate.split('/').pop()} surface${breaking ? ' declared breaking' : ''} answered ${answer.json.change} ${answer.json.next}, not ${change} ${next}`);
}
const named = decide('0.1.0', removed);
expect(JSON.stringify(named.json.removed ?? []).includes(removedName), `the breaking change did not name the removed ${removedName}`);
const invalid = decide('0.1', same);
expect(invalid.status !== 0, 'a current version that is not three slots was not refused');

if (broken.length > 0) {
  process.stderr.write(`${broken.join('\n')}\n`);
  process.exit(1 /* https://pubs.opengroup.org/onlinepubs/9799919799/functions/exit.html */);
}
process.stdout.write('autoversion classified every change by the surface and moved the slot the rule says\n');
