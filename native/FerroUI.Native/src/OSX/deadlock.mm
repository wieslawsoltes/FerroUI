#include "common.h"

static int Counter = 0;
FrnInsidePotentialDeadlock::FrnInsidePotentialDeadlock()
{
    Counter++;
}

FrnInsidePotentialDeadlock::~FrnInsidePotentialDeadlock()
{
    Counter--;
}

bool FrnInsidePotentialDeadlock::IsInside()
{
    return Counter!=0;
}
