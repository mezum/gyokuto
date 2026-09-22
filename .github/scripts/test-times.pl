#!/usr/bin/env perl
# cargo test の出力から、ビルド時間とテストバイナリごとの実行時間を Markdown の表にする
use strict;
use warnings;

my ($build, $name, $total, @rows) = ('?', '?', 0);
while (<>) {
    $build = $1 if /Finished `test` profile.* in (.+)$/;
    $name = "$2 ($1)" if /^\s*Running (\S+).*\W(\w+)-[0-9a-f]+(?:\.exe)?\)$/;
    $name = "$1 (doc)" if /^\s*Doc-tests (\S+)/;
    if (/^test result: \S+ (\d+) passed.* finished in ([\d.]+)s/) {
        $total += $2;
        push @rows, sprintf "| %s | %d | %.2fs |\n", $name, $1, $2;
    }
}
printf "ビルド: %s / テスト実行の合計: %.2fs\n\n", $build, $total;
print "| テスト | 件数 | 実行時間 |\n| --- | --- | ---: |\n", @rows;
