module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [
      2,
      'always',
      [
        'feat', 'fix', 'docs', 'chore',
        'refactor', 'perf', 'test', 'ci', 'style'
      ]
    ],
    'subject-empty': [2, 'never'],
    'type-empty': [2, 'never']
  }
};
