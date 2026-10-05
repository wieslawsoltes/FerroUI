//
//  FrnString.m
//  FerroUI.Native.OSX
//
//  Created by Dan Walmsley on 07/11/2018.
//

#include "common.h"
#include <vector>

class FrnStringImpl : public virtual ComSingleObject<IFrnString, &IID_IFrnString>
{
private:
    int _length;
    const char* _cstring;
    
public:
    FORWARD_IUNKNOWN()
    
    FrnStringImpl(NSString* string)
    { 
        auto cstring = [string cStringUsingEncoding:NSUTF8StringEncoding];
        _length = (int)[string lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
        
        _cstring = (const char*)malloc(_length + 5);
        
        memset((void*)_cstring, 0, _length + 5);
        memcpy((void*)_cstring, (void*)cstring, _length);
    }
    
    FrnStringImpl(void*ptr, int len)
    {
        _length = len;
        _cstring = (const char*)malloc(_length);
        memcpy((void*)_cstring, ptr, len);
    }
    
    virtual ~FrnStringImpl()
    {
        free((void*)_cstring);
    }
    
    virtual HRESULT Pointer(void**retOut) override
    {
        START_COM_CALL;
        
        @autoreleasepool
        {
            if(retOut == nullptr)
            {
                return E_POINTER;
            }
            
            *retOut = (void*)_cstring;
            
            return S_OK;
        }
    }
    
    virtual HRESULT Length(int*retOut) override
    {
        START_COM_CALL;
        
        @autoreleasepool
        {
            if(retOut == nullptr)
            {
                return E_POINTER;
            }
            
            *retOut = _length;
            
            return S_OK;
        }
    }
};

class FrnStringArrayImpl : public virtual ComSingleObject<IFrnStringArray, &IID_IFrnStringArray>
{
private:
    std::vector<ComPtr<FrnStringImpl>> _list;
public:
    FORWARD_IUNKNOWN()
    FrnStringArrayImpl(NSArray<NSString*>* array)
    {
        for(int c = 0; c < [array count]; c++)
        {
            _list.push_back(comnew<FrnStringImpl>([array objectAtIndex:c]));
        }
    }
    
    FrnStringArrayImpl(NSArray<NSURL*>* array)
    {
        for(int c = 0; c < [array count]; c++)
        {
            auto s = comnew<FrnStringImpl>([array objectAtIndex:c].absoluteString);
            _list.push_back(s);
        }
    }
    
    FrnStringArrayImpl(NSString* string)
    {
        _list.push_back(comnew<FrnStringImpl>(string));
    }
    
    virtual unsigned int GetCount() override
    {
        return (unsigned int)_list.size();
    }
    
    virtual HRESULT Get(unsigned int index, IFrnString**ppv) override
    {
        START_COM_CALL;
        
        @autoreleasepool
        {
            if(_list.size() <= index)
                return E_INVALIDARG;
            *ppv = _list[index].getRetainedReference();
            return S_OK;
        }
    }
};

IFrnString* CreateFrnString(NSString* string)
{
    return new FrnStringImpl(string);
}


IFrnStringArray* CreateFrnStringArray(NSArray<NSString*> * array)
{
    return new FrnStringArrayImpl(array);
}

IFrnStringArray* CreateFrnStringArray(NSArray<NSURL*> * array)
{
    return new FrnStringArrayImpl(array);
}

IFrnStringArray* CreateFrnStringArray(NSString* string)
{
    return new FrnStringArrayImpl(string);
}

IFrnString* CreateByteArray(void* data, int len)
{
    return new FrnStringImpl(data, len);
}

NSString* GetNSStringAndRelease(IFrnString* s)
{
    NSString* result = nil;
    
    if (s != nullptr)
    {
        char* p;
        if (s->Pointer((void**)&p) == S_OK && p != nullptr)
            result = [NSString stringWithUTF8String:p];
        
        s->Release();
    }
    
    return result;
}

NSString* GetNSStringWithoutRelease(IFrnString* s)
{
    NSString* result = nil;
    
    if (s != nullptr)
    {
        char* p;
        if (s->Pointer((void**)&p) == S_OK && p != nullptr)
            result = [NSString stringWithUTF8String:p];
    }
    
    return result;
}

NSArray<NSString*>* GetNSArrayOfStringsAndRelease(IFrnStringArray* array)
{
    auto output = [NSMutableArray array];
    if (array)
    {
        IFrnString* arrayItem;
        for (int i = 0; i < array->GetCount(); i++)
        {
            if (array->Get(i, &arrayItem) == 0) {
                NSString* ext = GetNSStringAndRelease(arrayItem);
                [output addObject:ext];
            }
        }
        array->Release();
    }
    return output;
}
