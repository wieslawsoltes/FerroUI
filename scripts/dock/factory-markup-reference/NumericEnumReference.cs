// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System;
using Dock.Model.Core;
using Dock.Settings;
namespace DockFactoryMarkupReference;
internal static class NumericEnumReference
{
    internal static void Run(Action<string,bool> check,Func<string,object> load)
    {
        var scope=new Avalonia.Controls.NameScope();object scalar=1;scope.Register("scalar",scalar);
        check("namescope-boxed-scalar-reference",ReferenceEquals(scope.Find("scalar"),scalar));
        check("numeric-enum-positive-overflow-narrowing",(int)((DockWindowOptions)load("CoreNumericPositiveOverflowRoot")).OwnerMode==int.MinValue);
        check("numeric-enum-negative-overflow-narrowing",(int)((DockWindowOptions)load("CoreNumericNegativeOverflowRoot")).OwnerMode==int.MaxValue);
        var options=(DockWindowOptions)load("CoreNumericWindowOptionsRoot");
        check("numeric-enum-options-owner-and-flags",(int)options.OwnerMode==-19&&options.IsModal&&options.ShowInTaskbar==false);
        var parent=options.ParentWindow!;
        check("numeric-enum-parent-boundaries",(int)parent.OwnerMode==int.MinValue&&(int)parent.WindowState==int.MaxValue&&parent.Topmost&&parent.Id=="numeric-parent");
        var window=(IDockWindow)load("CoreNumericWindowRoot");
        check("numeric-enum-window-values",(int)window.OwnerMode==123&&(int)window.WindowState==-17);
        options.ApplyTo(window);
        check("numeric-enum-original-apply-parent",ReferenceEquals(window.ParentWindow,parent)&&(int)window.OwnerMode==-19&&window.IsModal);
        var settings=(DockSettingsOptions)load("CoreNumericSettingsOptionsRoot");
        check("numeric-enum-nullable-option",(int?)settings.DefaultFloatingWindowOwnerMode==-19);
        var root=(Dock.Model.ReactiveUI.Controls.RootDock)load("CoreNumericDockEnumsRoot");
        check("numeric-dock-enum-original-model-values",(int)root.Dock==-19&&(int)root.PinnedDockDisplayMode==123&&(int?)root.PinnedDockDisplayModeOverride==int.MinValue&&(int)root.FloatingWindowHostMode==-17);
        var hostSettings=(DockSettingsOptions)load("CoreNumericHostSettingsRoot");
        check("numeric-dock-enum-nullable-host-option",(int?)hostSettings.FloatingWindowHostMode==-19);
        bool unset=true;
        foreach(var raw in new[]{0,-19,123,int.MinValue,int.MaxValue}) unset &= ReferenceEquals(Dock.Avalonia.Converters.DockModeConverter.Instance.Convert((DockMode)raw,typeof(Avalonia.Controls.Dock),null,System.Globalization.CultureInfo.InvariantCulture),Avalonia.AvaloniaProperty.UnsetValue);
        check("numeric-dock-mode-converter-source-fallback",unset);
        var previousMode=DockSettings.FloatingWindowHostMode;var previousManaged=DockSettings.UseManagedWindows;
        try {
            DockSettings.FloatingWindowHostMode=DockFloatingWindowHostMode.Managed;DockSettings.UseManagedWindows=true;
            check("numeric-host-mode-root-override",(int)DockSettings.ResolveFloatingWindowHostMode(root)==-17&&!DockSettings.IsManagedWindowHostingEnabled(root));
            root.FloatingWindowHostMode=DockFloatingWindowHostMode.Default;DockSettings.FloatingWindowHostMode=(DockFloatingWindowHostMode)123;
            check("numeric-host-mode-global-override",(int)DockSettings.ResolveFloatingWindowHostMode(root)==123&&!DockSettings.IsManagedWindowHostingEnabled(root));
            DockSettings.FloatingWindowHostMode=DockFloatingWindowHostMode.Default;
            check("numeric-host-mode-original-fallback",DockSettings.ResolveFloatingWindowHostMode(root)==DockFloatingWindowHostMode.Managed&&DockSettings.IsManagedWindowHostingEnabled(root));
        } finally {DockSettings.FloatingWindowHostMode=previousMode;DockSettings.UseManagedWindows=previousManaged;}
    }
}
