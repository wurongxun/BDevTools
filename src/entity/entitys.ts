export class SystemInfo {
    public memory_total: number | undefined;
    public memory_used: number | undefined ;
    public cpu_used: string | undefined ;
    public hostname: string | undefined ;
    public cpus: number | undefined;
}

export class TerminalInfo{
    public term:string='';
    public socketUri:string='' ;
    public socket:WebSocket ;
    public  accessToken:string='' ;
    // 构造函数 
    constructor(term:string,socketUri:string,socket:WebSocket,accessToken:string) { 
        this.term=term;
        this.socketUri=socketUri;
        this.socket=socket;
        this.accessToken=accessToken;
    }  
}

 class Message{
    public message_type:string='';
    public data: string='';
}

 class SshLinkInfo{
    public name:string='';
    public icon:string='';
    public path:string='';
    public children:SshLinkInfo[]=[];
    public parent:SshLinkInfo|null=null;
    public id:number=0;
    public type:string='';
    public isShow:boolean=false;
    public isHide:boolean=false;
    public isDisabled:boolean=false;
    public isDisabledChildren:boolean=false;
    public isDisabledParent:boolean=false;
}

class SettingMap{
    public id:string='';
    public app_setting_item_name:string='';
    public app_setting_item_value:string='';
}
class ResponseData<T> {
    public code:number | undefined;
    public data: T;
    public msg:string='';
    constructor(code:number,data:T,msg:string) {
        this.code=code;
        this.data=data;
        this.msg=msg;
    }
}
class SettingIdValueName<T> {
    public id:string;
    public app_setting_item_name: string;
    public app_setting_item_value:T;
    constructor(id:string,app_setting_item_name:string,app_setting_item_value:T) {
        this.id=id;
        this.app_setting_item_name=app_setting_item_name;
        this.app_setting_item_value=app_setting_item_value;
    }
}
class SshLinkInfoTree{
    public favorites_name:string=''; 
    public subMenu:SshSubMenu[]=[];
}
class SshSubMenu{
    public id:number=0;
    public name:string='';
    public address:string='';
    public port:number=22;
    public username:string=''; 
    public password:string=''; 
    public affix?:boolean=false;
    public agreement:string='';
    public favorites_name:string='';
}
export {Message,SshLinkInfo,SettingMap,ResponseData,SettingIdValueName,SshLinkInfoTree,SshSubMenu}