#pragma once
#include "ferro-native.h"
#include "FrnAccessibility.h"

// Defines a means for managed code to raise accessibility events.
class FrnAutomationNode : public ComSingleObject<IFrnAutomationNode, &IID_IFrnAutomationNode>
{
public:
    FORWARD_IUNKNOWN()
    FrnAutomationNode(id <FrnAccessibility> owner) { _owner = owner; }
    id <FrnAccessibility> GetOwner() { return _owner; }
    virtual void Dispose() override { _owner = nil; }
    virtual void ChildrenChanged () override { [_owner raiseChildrenChanged]; }
    virtual void PropertyChanged (FrnAutomationProperty property) override { [_owner raisePropertyChanged:property]; }
    virtual void FocusChanged () override { [_owner raiseFocusChanged]; }
private:
    // Native accessibility clients own the element. The notification node
    // must not keep that element, its peer and this node in a permanent cycle.
    __weak id <FrnAccessibility> _owner;
};
