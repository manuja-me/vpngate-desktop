using System;
using System.IO;
using System.Windows;
using System.Windows.Threading;

namespace VpnGate.Desktop;

public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        AppDomain.CurrentDomain.UnhandledException += (s, args) =>
        {
            LogCrash("AppDomain", args.ExceptionObject as Exception);
        };

        DispatcherUnhandledException += (s, args) =>
        {
            LogCrash("Dispatcher", args.Exception);
            args.Handled = true;
        };
    }

    private static void LogCrash(string source, Exception? ex)
    {
        try
        {
            var logPath = Path.Combine(Path.GetTempPath(), "vpngate_crash.log");
            File.AppendAllText(logPath, $"[{DateTime.UtcNow:O}] [{source}] {ex}\n\n");
        }
        catch { }
    }
}

