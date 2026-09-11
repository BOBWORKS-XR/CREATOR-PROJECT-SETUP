# Local child-process fixture: no downloads, installs, account or project access.
$mode = $args[0]
switch ($mode) {
    'query' {
        [Console]::Error.WriteLine('Dependency diagnostic separate from JSON')
        [Console]::Out.WriteLine('{"success":true,"data":{"totalDownloadSize":1234}}')
    }
    'bad-json' { [Console]::Out.WriteLine('not a JSON report') }
    'false-result' {
        [Console]::Out.WriteLine('{"type":"result","success":false}')
    }
    'offline' { exit 7 }
    'progress' {
        [Console]::Out.WriteLine('{"type":"progress","phase":"download","name":"Android","pct":42}')
        [Console]::Out.WriteLine('{"type":"progress","phase":"install","name":"Android","pct":50}')
        [Console]::Out.WriteLine('{"type":"result","success":true}')
    }
    default { exit 2 }
}
exit 0
