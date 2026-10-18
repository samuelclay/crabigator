import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import ts from 'typescript';

export function embedded(name, kind = 'js') {
    return runInNewContext(readFileSync(new URL(`../src/dashboard/${kind}/${name}.ts`, import.meta.url), 'utf8')
        .replace(/^export /m, '') + `\n${name.replace(/-([a-z])/g, (_, c) => c.toUpperCase())}${kind === 'js' ? 'Js' : 'Css'}`);
}

export function functions(code, names) {
    const source = ts.createSourceFile('dashboard.js', code, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
    return source.statements.filter(node => ts.isFunctionDeclaration(node) && names.includes(node.name?.text))
        .map(node => node.getText(source)).join('\n');
}

export const terminalUpdates = embedded('events').split('        function grokStatusLooksLike')[0];
