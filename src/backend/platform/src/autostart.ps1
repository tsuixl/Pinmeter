$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $taskName = 'Pinmeter-Autostart-' + $identity
    $scheduler = New-Object -ComObject 'Schedule.Service'
    $scheduler.Connect()
    $folder = $scheduler.GetFolder('\')
    $task = $null
    try { $task = $folder.GetTask($taskName) } catch {
        if ($_.Exception.HResult -ne -2147024894) { throw }
    }
    switch ($env:PINMETER_AUTOSTART_ACTION) {
        'checkpoint' {
            if ($null -ne $task) { [Console]::Write($task.Xml) }
        }
        'restore' {
            if ([string]::IsNullOrEmpty($env:PINMETER_AUTOSTART_CHECKPOINT)) {
                if ($null -ne $task) { $folder.DeleteTask($taskName, 0) }
            } else {
                $null = $folder.RegisterTask($taskName, $env:PINMETER_AUTOSTART_CHECKPOINT, 6, $identity, $null, 3)
            }
            [Console]::Write('ok')
        }
        'status' {
            if ($null -eq $task -or -not $task.Enabled) { [Console]::Write('disabled') }
            elseif ($task.Definition.Actions.Item(1).Path -ne $env:PINMETER_AUTOSTART_EXE) { [Console]::Write('other_path') }
            else { [Console]::Write('enabled') }
        }
        'enable' {
            $definition = $scheduler.NewTask(0)
            $definition.RegistrationInfo.Description = 'Start Pinmeter when this user signs in.'
            $definition.Principal.UserId = $identity
            $definition.Principal.LogonType = 3
            $definition.Principal.RunLevel = 1
            $definition.Settings.Enabled = $true
            $definition.Settings.DisallowStartIfOnBatteries = $false
            $definition.Settings.StopIfGoingOnBatteries = $false
            $definition.Settings.ExecutionTimeLimit = 'PT0S'
            $definition.Settings.MultipleInstances = 2
            $trigger = $definition.Triggers.Create(9)
            $trigger.UserId = $identity
            $trigger.Enabled = $true
            $action = $definition.Actions.Create(0)
            $action.Path = $env:PINMETER_AUTOSTART_EXE
            $action.WorkingDirectory = [IO.Path]::GetDirectoryName($env:PINMETER_AUTOSTART_EXE)
            $null = $folder.RegisterTaskDefinition($taskName, $definition, 6, $identity, $null, 3)
            [Console]::Write('ok')
        }
        'disable' {
            if ($null -ne $task) { $folder.DeleteTask($taskName, 0) }
            [Console]::Write('ok')
        }
        default { throw 'Invalid autostart action' }
    }
    exit 0
} catch {
    [Console]::Error.Write($_.Exception.Message)
    exit 1
}
