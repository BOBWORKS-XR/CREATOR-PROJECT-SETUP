using System;
using System.IO;
using System.Linq;
using BS.SDKEditor;
using Unity.VisualScripting;
using Unity.VisualScripting.Dependencies.Sqlite;
using UnityEditor;
using UnityEngine;
using UnityEngine.Rendering;

namespace CreatorWorks
{
    public static class ProjectSetupValidator
    {
        private static string StatusFolder => Path.Combine(Directory.GetParent(Application.dataPath).FullName, ".creator-project-setup");

        [Serializable]
        private sealed class Result
        {
            public bool success;
            public string unityVersion;
            public string creatorSdkVersion;
            public string urpVersion;
            public string inputSystemVersion;
            public bool urpConfigured;
            public bool androidSupported;
            public bool windowsSupported;
            public bool visualScriptingInitialized;
            public bool creatorVisualScriptingConfigured;
            public int nodeCount;
            public int creatorNodeCount;
            public string checkedAtUtc;
            public string error;
        }

        public static void Configure()
        {
            try
            {
                Progress(4, "Initializing Visual Scripting and saving Creator SDK type settings.");
                VSUsageUtility.isVisualScriptingUsed = true;
                var config = BoltCore.Configuration;
                config.assemblyOptions.Clear();
                config.assemblyOptions.AddRange(VsNodeGeneration.assemblyAllowList.Select(name => new LooseAssemblyName(name)));
                config.typeOptions.Clear();
                config.typeOptions.AddRange(VsNodeGeneration.typeAllowList);
                config.Save();
                config.SaveProjectSettingsAsset(true);
                Codebase.UpdateSettings();
                // Unity schedules settings serialization on delayCall, even with immediately=true.
                // Do not exit batch mode until that callback and the SDK's node generation finish.
                EditorApplication.delayCall += FinishConfiguration;
            }
            catch (Exception error) { Fail(error); }
        }

        private static void FinishConfiguration()
        {
            try
            {
                if (!File.Exists("ProjectSettings/VisualScriptingSettings.asset"))
                    throw new InvalidOperationException("Visual Scripting settings were not saved.");
                Progress(4, "Generating the Creator SDK Visual Scripting node database.");
                VsNodeGeneration.SetVSTypesAndAssemblies();
                AssetDatabase.SaveAssets();
                EditorApplication.Exit(0);
            }
            catch (Exception error) { Fail(error); }
        }

        // Run in a second Unity process: validate the settings actually loaded from disk,
        // not the in-memory configuration or a machine-wide SDK preference.
        public static void Validate()
        {
            try
            {
                Progress(5, "Checking the reloaded project, build platforms and Creator node database.");
                var result = new Result
                {
                    unityVersion = Application.unityVersion,
                    creatorSdkVersion = PackageVersion("com.sidequest.creator-sdk"),
                    urpVersion = PackageVersion("com.unity.render-pipelines.universal"),
                    inputSystemVersion = PackageVersion("com.unity.inputsystem"),
                    urpConfigured = GraphicsSettings.defaultRenderPipeline is UnityEngine.Rendering.Universal.UniversalRenderPipelineAsset,
                    androidSupported = BuildPipeline.IsBuildTargetSupported(BuildTargetGroup.Android, BuildTarget.Android),
                    windowsSupported = BuildPipeline.IsBuildTargetSupported(BuildTargetGroup.Standalone, BuildTarget.StandaloneWindows64),
                    visualScriptingInitialized = File.Exists("ProjectSettings/VisualScriptingSettings.asset") && VSUsageUtility.isVisualScriptingUsed && PluginContainer.initialized,
                    checkedAtUtc = DateTime.UtcNow.ToString("O")
                };
                if (result.visualScriptingInitialized)
                {
                    var config = BoltCore.Configuration;
                    result.creatorVisualScriptingConfigured = VsNodeGeneration.assemblyAllowList.All(name => config.assemblyOptions.Contains(new LooseAssemblyName(name)))
                        && VsNodeGeneration.typeAllowList.All(type => config.typeOptions.Contains(type));
                    if (File.Exists(BoltFlow.Paths.unitOptions))
                    {
                        using (NativeUtility.Module("sqlite3.dll"))
                        using (var database = new SQLiteConnection(BoltFlow.Paths.unitOptions, SQLiteOpenFlags.ReadOnly))
                        {
                            var nodes = database.Table<UnitOptionRow>().ToList();
                            result.nodeCount = nodes.Count;
                            result.creatorNodeCount = nodes.Count(row => row.unitType != null && row.unitType.StartsWith("BS.", StringComparison.Ordinal));
                        }
                    }
                }
                result.success = result.unityVersion == "@@EDITOR_VERSION@@"
                    && result.creatorSdkVersion == "@@CREATOR_SDK_VERSION@@"
                    && result.urpVersion == "@@URP_VERSION@@"
                    && result.inputSystemVersion == "@@INPUT_SYSTEM_VERSION@@"
                    && result.urpConfigured && result.androidSupported && result.windowsSupported
                    && result.visualScriptingInitialized && result.creatorVisualScriptingConfigured
                    && result.nodeCount > 0 && result.creatorNodeCount > 0;
                WriteResult(result);
                EditorApplication.Exit(result.success ? 0 : 2);
            }
            catch (Exception error) { Fail(error); }
        }

        private static string PackageVersion(string name) => UnityEditor.PackageManager.PackageInfo.FindForPackageName(name)?.version ?? "missing";

        [Serializable]
        private sealed class ProgressState { public int step; public string detail; }

        private static void Progress(int step, string detail)
        {
            Directory.CreateDirectory(StatusFolder);
            File.WriteAllText(Path.Combine(StatusFolder, "unity-progress.json"), JsonUtility.ToJson(new ProgressState { step = step, detail = detail }));
        }

        private static void WriteResult(Result result)
        {
            Directory.CreateDirectory(StatusFolder);
            File.WriteAllText(Path.Combine(StatusFolder, "unity-validation.json"), JsonUtility.ToJson(result, true));
        }

        private static void Fail(Exception error)
        {
            Debug.LogException(error);
            WriteResult(new Result { error = error.ToString(), checkedAtUtc = DateTime.UtcNow.ToString("O") });
            EditorApplication.Exit(2);
        }
    }
}
