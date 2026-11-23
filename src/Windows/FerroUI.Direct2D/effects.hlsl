// Small native kernels for the primitives Direct2D does not supply. Compiled
// by the Windows SDK at build time; no shader compiler ships in a normal app.
Texture2D<float4> source : register(t0);
Texture2D<float4> secondary : register(t1);
StructuredBuffer<float4> v : register(t2);
RWTexture2D<float4> output : register(u0);
SamplerState linearBorder : register(s0);
SamplerState linearClamp : register(s1);
cbuffer Parameters : register(b0) { uint kind, width, height, count; };
float lum(float3 c) { return dot(c,float3(0.3,0.59,0.11)); }
float3 clipColor(float3 c) {
    float l=lum(c),lo=min(c.r,min(c.g,c.b)),hi=max(c.r,max(c.g,c.b));
    if(lo<0)c=l+(c-l)*l/(l-lo);
    if(hi>1)c=l+(c-l)*(1-l)/(hi-l);
    return c;
}
float3 setLum(float3 c,float l) {return clipColor(c+l-lum(c));}
float sat(float3 c) {return max(c.r,max(c.g,c.b))-min(c.r,min(c.g,c.b));}
float3 setSat(float3 c,float s) {float lo=min(c.r,min(c.g,c.b)),hi=max(c.r,max(c.g,c.b));return hi>lo?(c-lo)*s/(hi-lo):0;}
float blendChannel(uint mode,float d,float s) {
    switch(mode) {
    case 1:return d*s;case 2:return d+s-d*s;case 3:return d<=0.5?2*d*s:1-2*(1-d)*(1-s);
    case 4:return min(d,s);case 5:return max(d,s);case 6:return min(1,d/max(1-s,1e-6));
    case 7:return 1-min(1,(1-d)/max(s,1e-6));
    case 8:{float g=d>0.25?sqrt(d):((16*d-12)*d+4)*d;return s<=0.5?d-(1-2*s)*d*(1-d):d+(2*s-1)*(g-d);}
    case 9:return s<=0.5?2*d*s:1-2*(1-d)*(1-s);case 10:return abs(d-s);case 11:return d+s-2*d*s;
    }return s;
}
float3 mixColor(float3 d,float3 s,uint mode) {
    if(mode==12)return setLum(setSat(s,sat(d)),lum(d));
    if(mode==13)return setLum(setSat(d,sat(s)),lum(d));
    if(mode==14)return setLum(s,lum(d));if(mode==15)return setLum(d,lum(s));
    return float3(blendChannel(mode,d.r,s.r),blendChannel(mode,d.g,s.g),blendChannel(mode,d.b,s.b));
}
float specialChannel(uint mode,float b,float s) {
    switch(mode) {
    case 6:return max(b+s-1,0);
    case 14:return s<=0.5?(b>=1?1:(s<=0?0:1-min((1-b)/(2*s),1))):(b<=0?0:(s>=1?1:min(b/(2*(1-s)),1)));
    case 15:return saturate(b+2*s-1);case 16:return s<=0.5?min(b,2*s):max(b,2*s-1);
    case 17:return b+s>=1?1:0;case 24:return max(b-s,0);case 25:return s<=0?(b<=0?0:1):min(b/s,1);
    }return s;
}
float4 pixelOperation(float4 d,float4 s,float2 pixel) {
    uint op=(uint)v[0].x;
    if(op==1){float4 c=saturate(v[1].x*d*s+v[1].y*d+v[1].z*s+v[1].w);c.rgb=min(c.rgb,c.a);return c;}
    if(op==2)return float4(s.a*(d.a-d.rgb)+(1-s.a)*d.rgb,d.a);
    uint mode=(uint)v[0].y;float3 cb=d.a>0?saturate(d.rgb/d.a):0,cs=s.a>0?saturate(s.rgb/s.a):0;
    if(mode==4) {
        float3 p=float3(pixel,1);float2 location=float2(dot(v[4].xyz,p),dot(v[5].xyz,p));
        float2 cell=clamp(floor((location-v[2].xy)*v[3].xy/v[2].zw),0,v[3].xy-1);
        uint hash=((uint)cell.y*(uint)v[3].x+(uint)cell.x)^0x9e3779b9;
        hash*=0x85ebca6b;hash^=hash>>13;hash*=0xc2b2ae35;hash^=hash>>16;
        return s.a>0&&(float)(hash&0xffff)/65535<s.a?float4(cs,1):d;
    }
    float3 blended;
    if(mode==7)blended=lum(cs)<lum(cb)?cs:cb;
    else if(mode==8)blended=lum(cs)>lum(cb)?cs:cb;
    else if(mode==10)blended=saturate(cb+cs);
    else if(mode==6||mode==14||mode==15||mode==16||mode==17||mode==24||mode==25)blended=float3(specialChannel(mode,cb.r,cs.r),specialChannel(mode,cb.g,cs.g),specialChannel(mode,cb.b,cs.b));
    else {
        uint cg=0;
        switch(mode){case 0:cg=1;break;case 1:cg=2;break;case 2:cg=4;break;case 3:cg=5;break;case 5:cg=7;break;case 9:cg=6;break;case 11:cg=3;break;case 12:cg=8;break;case 13:cg=9;break;case 18:cg=10;break;case 19:cg=11;break;case 20:cg=12;break;case 21:cg=13;break;case 22:cg=14;break;case 23:cg=15;break;}
        blended=mixColor(cb,cs,cg);
    }
    return float4((1-s.a)*d.rgb+(1-d.a)*s.rgb+s.a*d.a*blended,s.a+d.a*(1-s.a));
}
float3 lightAt(float2 pixel) {
    float3 location=float3(pixel,1);float2 local=float2(dot(v[4].xyz,location),dot(v[5].xyz,location));
    float3 result=v[0].rgb;float4 material=v[1];
    for(uint i=0;i<min((uint)v[2].z,8);++i) {
        uint at=6+i*10;float3 q=float3(local,1);float w=dot(v[at+9].xyz,q);if(abs(w)<1e-12)continue;
        float2 p=float2(dot(v[at+7].xyz,q),dot(v[at+8].xyz,q))/w;
        float3 toLight=float3(v[at+3].xy-p,v[at+3].z);float distance=length(toLight);
        if(v[at+6].y>0&&distance>v[at+6].x)continue;
        float d=max(max(distance,v[at+5].w),0);
        float denominator=v[at+5].x+v[at+5].y*d+v[at+5].z*d*d;float attenuation=denominator>0?1/denominator:1;
        float incidence=distance>0&&v[at+3].z>0?v[at+3].z/distance:0;
        float cone=1;float3 color=v[at+1].rgb;
        if(v[at].x>1.5) {
            float3 direction=v[at+4].xyz;float a=length(direction)*distance;if(a<=0)continue;
            float c=dot(-toLight,direction)/a;
            if(c>=v[at].y)cone=1;else if(c<=v[at].z||v[at].y<=v[at].z)continue;
            else {float t=saturate((c-v[at].z)/(v[at].y-v[at].z));cone=t*t*(3-2*t);}
            color=lerp(v[at+2].rgb,v[at+1].rgb,cone);
        }
        float specular=0;
        if(distance>0&&toLight.z>0){float3 h=toLight/distance+float3(0,0,1);float len=length(h);if(len>0)specular=pow(max(h.z/len,0),max(material.w,1));}
        result+=color*cone*attenuation*(incidence*material.y+specular*material.z);
    }return result;
}
float4 conic(float2 pixel) {
    float3 p=float3(pixel,1);float2 local=float2(dot(v[0].xyz,p),dot(v[1].xyz,p));
    float2 q=local-v[2].xy;float t=frac(atan2(q.x,-q.y)/(2*3.141592653589793)-v[2].z/360+1);
    uint stops=(uint)v[2].w;float4 c=v[4];
    for(uint i=1;i<stops;++i){float offset=v[5+i*2].x;if(t<=offset){float previous=v[5+(i-1)*2].x;float f=offset>previous?saturate((t-previous)/(offset-previous)):1;c=lerp(v[4+(i-1)*2],v[4+i*2],f);c.rgb*=c.a;return c;}}
    c=v[4+(stops-1)*2];c.rgb*=c.a;return c;
}
[numthreads(8,8,1)]
void main(uint3 id:SV_DispatchThreadID) {
    uint2 p=id.xy;if(p.x>=width||p.y>=height)return;
    float2 pixel=float2(p)+0.5;float4 color=source.Load(int3(p,0));
    if(kind==3) {
        float2 local=float2(v[0].x*pixel.x+v[0].z*pixel.y,v[0].y*pixel.x+v[0].w*pixel.y)+v[1].xy;
        float2 uv=(local-v[2].xy)/v[2].zw;float3 factors=secondary.SampleLevel(linearClamp,uv,0).rgb*v[1].z;
        color.rgb=min(color.rgb*factors,color.a);if(any(uv<0)||any(uv>1))color=0;
    } else if(kind==4) {
        uint kw=(uint)v[0].x,kh=(uint)v[0].y;float4 sum=0;int4 bounds=(int4)v[2];
        for(uint y=0;y<kh;++y)for(uint x=0;x<kw;++x) {
            int2 at=(int2)p+int2(x,y)-int2(kw/2,kh/2);
            if(v[1].x>0)at=clamp(at,bounds.xy,bounds.zw-1);
            float4 sample=0;
            if(all(at>=bounds.xy)&&all(at<bounds.zw)&&all(at>=0)&&at.x<(int)width&&at.y<(int)height)sample=source.Load(int3(at,0));
            if(v[1].y>0&&sample.a>0)sample.rgb/=sample.a;
            uint index=y*kw+x;sum+=sample*v[3+index/4][index%4];
        }
        sum=sum*v[0].z+v[0].w;
        if(v[1].y>0){sum.a=color.a;sum.rgb=saturate(sum.rgb)*color.a;}
        else {sum=saturate(sum);sum.rgb=min(sum.rgb,sum.a);}
        color=sum;
    } else if(kind==5) {
        float alpha=source.SampleLevel(linearBorder,(pixel-v[0].xy)/float2(width,height),0).a;
        float mask=secondary.Load(int3(p,0)).a;alpha=(v[0].z>0?(1-alpha)*mask:alpha*(1-mask))*v[1].a;
        color=float4(v[1].rgb*alpha,alpha);
    } else if(kind==6) {
        float3 light=lightAt(pixel);if(v[2].x>0)color.rgb=min(color.rgb*clamp(light,0,v[2].y),color.a);else color=float4(saturate(light),1);
    } else if(kind==7)color=pixelOperation(secondary.Load(int3(p,0)),color,pixel);
    else if(kind==8)color=conic(pixel);
    output[p]=color;
}
