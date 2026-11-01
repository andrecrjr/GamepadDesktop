// @ts-check
import { appendFileSync } from 'node:fs';
import semanticRelease from 'semantic-release';

const output = process.env.GITHUB_OUTPUT;
if (!output) throw new Error('GITHUB_OUTPUT is required for release planning');
const result = await semanticRelease({ dryRun: true });
const version = result && result.nextRelease ? result.nextRelease.version : '';
appendFileSync(output, `version=${version}\n`);
