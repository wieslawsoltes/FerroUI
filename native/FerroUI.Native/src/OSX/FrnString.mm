//
//  FrnString.m
//  FerroUI.Native.OSX
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
    
    virtual HRESULT Get(unsigned int index, ComPtr<IFrnString>& ppv) override
    {
        if(_list.size() <= index)
            return E_INVALIDARG;
        ppv = _list[index];
        return S_OK;
    }
};

ComPtr<IFrnString> CreateFrnString(NSString* string)
{
    return comnew<FrnStringImpl>(string);
}

ComPtr<IFrnStringArray> CreateFrnStringArray(NSArray<NSString*> * array)
{
    return comnew<FrnStringArrayImpl>(array);
}

ComPtr<IFrnStringArray> CreateFrnStringArray(NSArray<NSURL*> * array)
{
    return comnew<FrnStringArrayImpl>(array);
}

ComPtr<IFrnStringArray> CreateFrnStringArray(NSString* string)
{
    return comnew<FrnStringArrayImpl>(string);
}

ComPtr<IFrnString> CreateByteArray(void* data, int len)
{
    return comnew<FrnStringImpl>(data, len);
}

NSString* GetNSString(const ComPtr<IFrnString>& s)
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

NSArray<NSString*>* GetNSArrayOfStrings(const ComPtr<IFrnStringArray>& array)
{
    auto output = [NSMutableArray array];
    if (array)
    {
        for (int i = 0; i < array->GetCount(); i++)
        {
            ComPtr<IFrnString> arrayItem;
            if (array->Get(i, arrayItem) == 0) {
                NSString* ext = GetNSString(arrayItem);
                [output addObject:ext];
            }
        }
    }
    return output;
}
